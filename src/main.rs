use futures_util::{ StreamExt, io::{ AsyncBufReadExt, BufReader, AllowStdIo }};
use std::io::{ Write, Error as IoError, ErrorKind, Cursor };
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use serde_json::Value;
use termcolor::{BufferWriter, Buffer, Color, ColorChoice, ColorSpec, WriteColor};
pub type Error = Box<dyn std::error::Error + Send + Sync>;



#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
pub async fn main() -> Result<(), std::io::Error> {
    let stdin = AllowStdIo::new(std::io::stdin());
    let mut lines = BufReader::new(stdin).lines();
    let (tx, mut rx) = channel::<Buffer>(100);
    let color = if atty::is(atty::Stream::Stdout) {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };
    let bufwtr = Arc::new(BufferWriter::stdout(color));
    let writer = bufwtr.clone();

    tokio::spawn(async move {
        while let Some(buf) = rx.recv().await {
            writer.print(&buf).expect("writer print");
        }
        Ok::<_, std::io::Error>(())
    });
    while let Some(line) = lines.next().await {
        let line = line?;
        if let Ok(record) = serde_json::from_str(&line) {
            let mut buf = bufwtr.buffer();
            process_record(record, &mut buf).expect("process_record");
            let _ = tx.send(buf).await;
        } else {
            let mut buf = bufwtr.buffer();
            buf.write(line.as_bytes()).expect("buf write");
            buf.write(b"\n")?;
            let _ = tx.send(buf).await;
        }
    }
    Ok(())
}

fn process_record(mut rec: Value, outbuf: &mut Buffer) -> Result<(), std::io::Error> {
    if let Some(record) = rec.as_object_mut() {
        if let Some(ts) = record.get("ts") {
            if let Some(ts) = ts.as_str() {
                outbuf.write(b"[")?;
                outbuf.set_color(ColorSpec::new().set_intense(true).set_fg(Some(Color::White)))?;
                outbuf.write(ts.as_bytes())?;
                outbuf.set_color(&ColorSpec::new())?;
                outbuf.write(b"]")?;
                record.remove("ts");
            }
        }
        if let Some(level) = record.get("level") {
            if let Some(level) = level.as_u64() {
                match level {
                    10 => {
                        outbuf.write(b"[")?;
                        outbuf.set_color(&ColorSpec::new())?;
                        outbuf.write(b"TRACE")?;
                        outbuf.write(b"]")?;
                        record.remove("level");
                    }
                    20 => {
                        outbuf.write(b"[")?;
                        outbuf.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                        outbuf.write(b"DEBUG")?;
                        outbuf.set_color(&ColorSpec::new())?;
                        outbuf.write(b"]")?;
                        record.remove("level");
                    }
                    30 => {
                        outbuf.write(b"[")?;
                        outbuf.set_color(ColorSpec::new().set_intense(true).set_fg(Some(Color::Green)))?;
                        outbuf.write(b" INFO")?;
                        outbuf.set_color(&ColorSpec::new())?;
                        outbuf.write(b"]")?;
                        record.remove("level");
                    }
                    40 => {
                        outbuf.write(b"[")?;
                        outbuf.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                        outbuf.write(b" WARN ")?;
                        outbuf.set_color(&ColorSpec::new())?;
                        outbuf.write(b"]")?;
                        record.remove("level");
                    }
                    50 => {
                        outbuf.write(b"[")?;
                        outbuf.set_color(ColorSpec::new().set_intense(true).set_fg(Some(Color::Yellow)))?;
                        outbuf.write(b"ERROR")?;
                        outbuf.set_color(&ColorSpec::new())?;
                        outbuf.write(b"]")?;
                        record.remove("level");
                    }
                    60 => {
                        outbuf.write(b"[")?;
                        outbuf.set_color(ColorSpec::new().set_intense(true).set_fg(Some(Color::Magenta)))?;
                        outbuf.write(b"ALERT")?;
                        outbuf.set_color(&ColorSpec::new())?;
                        outbuf.write(b"]")?;
                        record.remove("level");
                    }
                    70 => {
                        outbuf.write(b"[")?;
                        outbuf.set_color(ColorSpec::new().set_intense(true).set_fg(Some(Color::Red)))?;
                        outbuf.write(b"FATAL")?;
                        outbuf.set_color(&ColorSpec::new())?;
                        outbuf.write(b"]")?;
                        record.remove("level");
                    }
                    _ => {}
                }
            }
        }
        if let Some(trace) = record.get("svc") {
            if let Some(trace) = trace.as_str() {
                outbuf.write(b"[")?;
                outbuf.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                outbuf.write(trace.as_bytes())?;
                outbuf.set_color(&ColorSpec::new())?;
                outbuf.write(b"]")?;
                record.remove("svc");
            }
        }
        if let Some(trace) = record.get("traceId") {
            if let Some(trace) = trace.as_str() {
                outbuf.write(b"[")?;
                outbuf.set_color(ColorSpec::new().set_intense(true).set_fg(Some(Color::Blue)))?;
                outbuf.write(trace.as_bytes())?;
                outbuf.set_color(&ColorSpec::new())?;
                outbuf.write(b"]")?;
                record.remove("traceId");
            }
        }
        if let Some(msg) = record.get("event") {
            if let Some(msg) = msg.as_str() {
                outbuf.write(b" event=")?;
                outbuf.write(msg.as_bytes())?;
                record.remove("event");
            }
        }
        if let Some(msg) = record.get("message") {
            if let Some(msg) = msg.as_str() {
                outbuf.write(b" message=")?;
                outbuf.write(msg.as_bytes())?;
                record.remove("message");
            }
        }
        record.iter().for_each(|(k,v)| {
            let _ = outbuf.write(b" ");
            let _ = outbuf.write(k.as_bytes());
            let _ = outbuf.write(b"=");
            let _ = outbuf.write(rec_to_bytes(v).unwrap().as_slice());
        });
        outbuf.write(b"\n")?;
        Ok(())
    } else {
        println!("rec: {:?}", rec);
        Err(IoError::new(ErrorKind::Other, "not a javascript object"))
    }
}

fn rec_to_bytes(value: &Value) -> Result<Vec<u8>, IoError> {
    match value {
        Value::Object(m) => {
            let mut res = Cursor::new(vec![]);
            for (k, v) in m.iter() {
                res.write(b"\n    ")?;
                res.write(k.as_bytes())?;
                res.write(b"=")?;
                let o = serde_json::to_vec(v).map_err(|e| IoError::new(ErrorKind::Other, format!("{}", e)))?;
                res.write(o.as_slice())?;
            }
            res.write(b"\n")?;
            Ok(res.into_inner())
        },
        _ => serde_json::to_vec(value).map_err(|e| IoError::new(ErrorKind::Other, format!("{}", e)))
    }
}
