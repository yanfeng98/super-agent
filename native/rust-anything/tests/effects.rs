mod common;

use std::path::Path;

use rust_anything::app::{self, RecordOptions, ReplayOptions};
use rust_anything::effects::EffectsConfig;

const NOTIFY: &str = r#"import json, os, smtplib, sys, urllib.request
from email.message import EmailMessage

name = sys.argv[1] if len(sys.argv) > 1 else "nobody"
times = int(os.environ.get("MAILS", "1"))
req = urllib.request.Request(
    os.environ["WEBHOOK_URL"] + "/signup",
    data=json.dumps({"user": name}).encode(),
    headers={"Content-Type": "application/json"},
    method="POST",
)
urllib.request.urlopen(req).read()
for _ in range(times):
    m = EmailMessage()
    m["From"] = "app@example.test"
    m["To"] = name + "@example.test"
    m["Subject"] = "welcome"
    m.set_content("hi " + name)
    with smtplib.SMTP(os.environ["SMTP_HOST"], int(os.environ["SMTP_PORT"]), local_hostname="localhost") as s:
        s.send_message(m)
print("signed up", name)
"#;

fn record(repo: &Path, python: &str, effects: Option<EffectsConfig>) -> app::RecordOutcome {
    app::record(&RecordOptions {
        repo,
        run: &[python.to_string(), "notify.py".to_string()],
        inputs: Some(&repo.join("inputs.jsonl")),
        normalize: &[],
        name: Some("notify"),
        traps: false,
        timeout_secs: None,
        http: false,
        server_args: &[],
        fresh: &[],
        pace_ms: None,
        port: None,
        float_tolerance: None,
        float_files: &[],
        effects,
    })
    .unwrap()
}

fn replay(repo: &Path, build: &str) -> app::ReplayOutcome {
    app::replay(&ReplayOptions {
        repo,
        build: Some(build),
        timeout_secs: None,
        port: None,
        case: None,
    })
    .unwrap()
}

#[test]
fn a_second_email_fails_replay_even_when_the_output_matches() {
    let Some(python) = common::python() else {
        return;
    };
    let repo = std::env::temp_dir().join(format!("rust-anything-effects-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("notify.py"), NOTIFY).unwrap();
    std::fs::write(
        repo.join("twice.py"),
        NOTIFY.replace("\"MAILS\", \"1\"", "\"MAILS\", \"2\""),
    )
    .unwrap();
    std::fs::write(
        repo.join("inputs.jsonl"),
        "{\"name\": \"alice\", \"args\": [\"alice\"]}\n{\"name\": \"bob\", \"args\": [\"bob\"]}\n",
    )
    .unwrap();

    let effects = EffectsConfig::from_args(
        &["smtp".into()],
        &["WEBHOOK_URL=http-url".into()],
        None,
        None,
    )
    .unwrap();
    let outcome = record(&repo, python, effects);
    assert!(outcome.cases >= 2, "{outcome:?}");
    assert!(outcome.unstable.is_empty(), "{outcome:?}");

    let recorded = std::fs::read_to_string(app::cases_dir(&repo).join("alice.effects")).unwrap();
    assert!(
        recorded.starts_with("http POST http://sink/signup\ncontent-type: application/json\n"),
        "{recorded}"
    );
    assert!(recorded.contains("{\"user\": \"alice\"}"), "{recorded}");
    assert_eq!(
        recorded
            .matches("smtp from <app@example.test> to <alice@example.test>")
            .count(),
        1,
        "{recorded}"
    );

    let same = replay(&repo, &format!("{python} notify.py"));
    assert_eq!(same.passed, same.total, "{:?}", same.failures);

    let twice = replay(&repo, &format!("{python} twice.py"));
    assert_eq!(twice.passed, 0, "{twice:?}");
    for failure in &twice.failures {
        let fields: Vec<&str> = failure.problems.iter().map(|p| p.field.as_str()).collect();
        assert_eq!(fields, ["effects"], "{failure:?}");
    }

    std::fs::remove_dir_all(&repo).unwrap();
}

const SERVER: &str = r#"import json, os, smtplib
from email.message import EmailMessage
from http.server import BaseHTTPRequestHandler, HTTPServer

MAILS = int(os.environ.get("MAILS", "1"))

class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def reply(self, code, body):
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self.reply(200, {"ok": True})

    def do_POST(self):
        n = int(self.headers.get("Content-Length", "0"))
        user = json.loads(self.rfile.read(n) or b"{}").get("user", "nobody")
        for _ in range(MAILS):
            m = EmailMessage()
            m["From"] = "app@example.test"
            m["To"] = user + "@example.test"
            m["Subject"] = "welcome"
            m.set_content("hi " + user)
            with smtplib.SMTP(os.environ["SMTP_HOST"], int(os.environ["SMTP_PORT"]), local_hostname="localhost") as s:
                s.send_message(m)
        self.reply(201, {"user": user})

HTTPServer(("127.0.0.1", int(os.environ["PORT"])), H).serve_forever()
"#;

#[test]
fn a_server_that_mails_twice_fails_only_on_that_request() {
    let Some(python) = common::python() else {
        return;
    };
    let repo =
        std::env::temp_dir().join(format!("rust-anything-effects-http-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("server.py"), SERVER).unwrap();
    std::fs::write(
        repo.join("twice.py"),
        SERVER.replace("\"MAILS\", \"1\"", "\"MAILS\", \"2\""),
    )
    .unwrap();
    std::fs::write(
        repo.join("inputs.jsonl"),
        "{\"name\": \"health\", \"method\": \"GET\", \"path\": \"/health\"}\n\
         {\"name\": \"signup\", \"method\": \"POST\", \"path\": \"/signup\", \"headers\": {\"Content-Type\": \"application/json\"}, \"body\": \"{\\\"user\\\": \\\"carol\\\"}\"}\n",
    )
    .unwrap();
    let outcome = app::record(&RecordOptions {
        repo: &repo,
        run: &[python.to_string(), "server.py".to_string()],
        inputs: Some(&repo.join("inputs.jsonl")),
        normalize: &[],
        name: Some("server"),
        traps: false,
        timeout_secs: None,
        http: true,
        server_args: &[],
        fresh: &[],
        pace_ms: None,
        port: None,
        float_tolerance: None,
        float_files: &[],
        effects: EffectsConfig::from_args(&["smtp".into()], &[], None, None).unwrap(),
    })
    .unwrap();
    assert_eq!(outcome.cases, 2, "{outcome:?}");
    let http_dir = app::cases_dir(&repo).join("http");
    assert_eq!(
        std::fs::read_to_string(http_dir.join("health.effects")).unwrap(),
        ""
    );
    let mail = std::fs::read_to_string(http_dir.join("signup.effects")).unwrap();
    assert!(
        mail.starts_with("smtp from <app@example.test> to <carol@example.test>\n"),
        "{mail}"
    );

    let same = replay(&repo, &format!("{python} server.py"));
    assert_eq!(same.passed, same.total, "{:?}", same.failures);
    let twice = replay(&repo, &format!("{python} twice.py"));
    assert_eq!((twice.passed, twice.total), (1, 2), "{twice:?}");
    assert_eq!(twice.failures[0].case, "signup");
    assert_eq!(twice.failures[0].problems[0].field, "effects");

    std::fs::remove_dir_all(&repo).unwrap();
}

const DB_APP: &str = r#"import os, sqlite3, sys
db = sqlite3.connect(os.path.join(os.path.dirname(os.path.abspath(__file__)), "app.db"))
name = sys.argv[1] if len(sys.argv) > 1 else "nobody"
for _ in range(int(os.environ.get("ROWS", "1"))):
    db.execute("insert into users(name) values (?)", (name,))
db.commit()
print("added", name)
"#;

#[test]
fn rows_written_differently_fail_replay_through_the_db_dump() {
    let Some(python) = common::python() else {
        return;
    };
    let repo =
        std::env::temp_dir().join(format!("rust-anything-effects-db-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("add.py"), DB_APP).unwrap();
    std::fs::write(
        repo.join("add_twice.py"),
        DB_APP.replace("\"ROWS\", \"1\"", "\"ROWS\", \"2\""),
    )
    .unwrap();
    std::fs::write(
        repo.join("reset.py"),
        "import sqlite3\ndb = sqlite3.connect('app.db')\ndb.execute('drop table if exists users')\ndb.execute('create table users(id integer primary key, name text)')\ndb.commit()\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("dump.py"),
        "import sqlite3\nfor row in sqlite3.connect('app.db').execute('select id, name from users order by id'):\n    print(*row)\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("inputs.jsonl"),
        "{\"name\": \"dave\", \"args\": [\"dave\"]}\n",
    )
    .unwrap();
    let effects = EffectsConfig::from_args(
        &[],
        &[],
        Some(&format!("{python} reset.py")),
        Some(&format!("{python} dump.py")),
    )
    .unwrap();
    let outcome = app::record(&RecordOptions {
        repo: &repo,
        run: &[python.to_string(), "add.py".to_string()],
        inputs: Some(&repo.join("inputs.jsonl")),
        normalize: &[],
        name: Some("add"),
        traps: false,
        timeout_secs: None,
        http: false,
        server_args: &[],
        fresh: &[],
        pace_ms: None,
        port: None,
        float_tolerance: None,
        float_files: &[],
        effects,
    })
    .unwrap();
    assert!(outcome.unstable.is_empty(), "{outcome:?}");
    assert_eq!(
        std::fs::read_to_string(app::cases_dir(&repo).join("dave.effects")).unwrap(),
        "db\n1 dave\n"
    );
    let same = replay(&repo, &format!("{python} add.py"));
    assert_eq!(same.passed, same.total, "{:?}", same.failures);
    let twice = replay(&repo, &format!("{python} add_twice.py"));
    let dave = twice.failures.iter().find(|f| f.case == "dave").unwrap();
    assert_eq!(dave.problems[0].field, "effects", "{dave:?}");

    std::fs::remove_dir_all(&repo).unwrap();
}
