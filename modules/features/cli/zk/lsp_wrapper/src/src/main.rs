use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

type Requests = Arc<Mutex<HashMap<String, String>>>;

fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Value>> {
    let mut content_length = None;

    loop {
        let mut line = String::new();

        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }

        let line = line.trim_end_matches(['\r', '\n']);

        if line.is_empty() {
            break;
        }

        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                content_length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                );
            }
        }
    }

    let length = content_length.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "LSP message has no Content-Length",
        )
    })?;

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;

    let message = serde_json::from_slice(&body)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(Some(message))
}

fn write_message<W: Write>(writer: &mut W, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    write!(
        writer,
        "Content-Length: {}\r\n\
         Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n\
         \r\n",
        body.len()
    )?;

    writer.write_all(&body)?;
    writer.flush()
}

fn filename_only(text: &str) -> String {
    if !text.starts_with("[[") || !text.ends_with("]]") {
        return text.to_owned();
    }

    let target = &text[2..text.len() - 2];

    // zk uses POSIX-style paths in the completion text.
    let filename = target.rsplit('/').next().unwrap_or(target);

    // Equivalent to Python's PurePosixPath(...).stem:
    // remove the final extension.
    let filename = filename
        .strip_suffix(
            filename
                .rsplit_once('.')
                .map(|(_, ext)| format!(".{ext}"))
                .as_deref()
                .unwrap_or(""),
        )
        .unwrap_or(filename);

    format!("[[{filename}]]")
}

fn rewrite_item(item: &mut Value) {
    let Some(object) = item.as_object_mut() else {
        return;
    };

    if let Some(edit) = object.get_mut("textEdit") {
        if let Some(edit_object) = edit.as_object_mut() {
            if let Some(new_text) = edit_object.get_mut("newText") {
                if let Some(text) = new_text.as_str() {
                    *new_text = Value::String(filename_only(text));
                }
            }
        }
    } else if let Some(insert_text) = object.get_mut("insertText") {
        if let Some(text) = insert_text.as_str() {
            *insert_text = Value::String(filename_only(text));
        }
    }
}

fn rewrite_completion(result: &mut Value) {
    match result {
        Value::Array(items) => {
            for item in items {
                rewrite_item(item);
            }
        }

        Value::Object(object) => {
            if let Some(Value::Array(items)) = object.get_mut("items") {
                for item in items {
                    rewrite_item(item);
                }
            }
        }

        _ => {}
    }
}

fn client_to_zk(
    mut zk_stdin: impl Write + Send + 'static,
    requests: Requests,
) -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        let Some(message) = read_message(&mut reader)? else {
            return Ok(());
        };

        if let (Some(id), Some(method)) = (message.get("id"), message.get("method")) {
            if let Some(method) = method.as_str() {
                let key = serde_json::to_string(id).unwrap();

                requests
                    .lock()
                    .unwrap()
                    .insert(key, method.to_owned());
            }
        }

        write_message(&mut zk_stdin, &message)?;
    }
}

fn zk_to_client(
    mut zk_stdout: impl BufRead,
    requests: Requests,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    loop {
        let Some(mut message) = read_message(&mut zk_stdout)? else {
            return Ok(());
        };

        if let Some(id) = message.get("id") {
            let is_response = message.get("method").is_none();

            if is_response {
                let key = serde_json::to_string(id).unwrap();

                let method = requests.lock().unwrap().remove(&key);

                if method.as_deref() == Some("textDocument/completion") {
                    if let Some(result) = message.get_mut("result") {
                        rewrite_completion(result);
                    }
                } else if method.as_deref() == Some("completionItem/resolve") {
                    if let Some(result) = message.get_mut("result") {
                        rewrite_item(result);
                    }
                }
            }
        }

        write_message(&mut writer, &message)?;
    }
}

fn main() -> io::Result<()> {
    let mut zk = Command::new("zk")
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let zk_stdin = zk.stdin.take().expect("zk stdin");
    let zk_stdout = zk.stdout.take().expect("zk stdout");

    let requests: Requests = Arc::new(Mutex::new(HashMap::new()));

    let requests_from_client = Arc::clone(&requests);
    let client_thread = thread::spawn(move || {
        if let Err(error) = client_to_zk(zk_stdin, requests_from_client) {
            eprintln!("zk-lsp-helix: client -> zk: {error}");
        }
    });

    let requests_from_zk = Arc::clone(&requests);
    let server_thread = thread::spawn(move || {
        let reader = io::BufReader::new(zk_stdout);

        if let Err(error) = zk_to_client(reader, requests_from_zk) {
            eprintln!("zk-lsp-helix: zk -> client: {error}");
        }
    });

    let _ = client_thread.join();
    let _ = server_thread.join();

    let _ = zk.kill();

    Ok(())
}
