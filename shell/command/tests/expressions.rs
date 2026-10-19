use std::fs;
use std::process::Command as Process;

use tta_command::{And, ChangeDir, Command, Expression, Handoff, Word};

fn command(program: &str, args: &[&str]) -> Expression {
    Command::builder()
        .program(Word::new(program).unwrap())
        .args(args.iter().map(|arg| Word::new(*arg).unwrap()).collect())
        .build()
        .into()
}

#[test]
fn quoting_round_trips_hostile_and_empty_operands() {
    let words = [
        "",
        "plain",
        "a'b",
        "a\nb\r\tc",
        "$HOME; $(exit 9) `exit 8`",
        "雪",
        "\\",
        "-n",
        "#{session_name}",
    ];
    for word in words {
        let script = format!("printf '%s' {}", Word::new(word).unwrap().shell().unwrap());
        let output = Process::new("sh").args(["-c", &script]).output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, word.as_bytes());
    }
    assert!(Word::new("a\0b").is_err());
}

#[test]
fn direct_and_shell_share_argv_and_physical_cwd() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path().join("quotes ' space $; 雪");
    fs::create_dir(&cwd).unwrap();
    // A tiny external recorder; the expression itself does not need a shell.
    let script = temp.path().join("record.py");
    fs::write(
        &script,
        "import os, sys, json\nopen(sys.argv[1], 'w').write(json.dumps([os.getcwd(), sys.argv[2:]]))\n",
    )
    .unwrap();
    let output = temp.path().join("result");
    let expression: Expression = And::builder()
        .steps(vec![
            ChangeDir::builder().path(Word::new(cwd.as_os_str()).unwrap()).build().into(),
            Command::builder()
                .program(Word::new("python3").unwrap())
                .args(
                    [script.as_os_str(), output.as_os_str(), "a'b".as_ref(), "".as_ref(), "x;y\n$z".as_ref()]
                        .into_iter()
                        .map(|word| Word::new(word).unwrap())
                        .collect(),
                )
                .build()
                .into(),
        ])
        .build()
        .into();
    assert!(expression.execute(Handoff::Run).unwrap().unwrap().success());
    let direct = fs::read(&output).unwrap();
    fs::remove_file(&output).unwrap();
    let status = Process::new("sh").args(["-c", &expression.shell(Handoff::Run).unwrap()]).status().unwrap();
    assert!(status.success());
    assert_eq!(fs::read(output).unwrap(), direct);
    assert!(!expression.shell(Handoff::Run).unwrap().contains("exec "));
    assert!(expression.shell(Handoff::Exec).unwrap().contains("&& exec "));
}

#[test]
fn and_stops_on_failure_in_both_modes() {
    let expression: Expression = And::builder().steps(vec![command("false", &[]), command("true", &[])]).build().into();
    assert!(!expression.execute(Handoff::Run).unwrap().unwrap().success());
    assert!(
        !Process::new("sh")
            .args(["-c", &expression.shell(Handoff::Run).unwrap()])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn failed_cd_does_not_run_command() {
    let temp = tempfile::tempdir().unwrap();
    let expression: Expression = And::builder()
        .steps(vec![
            ChangeDir::builder().path(Word::new(temp.path().join("missing")).unwrap()).build().into(),
            command("true", &[]),
        ])
        .build()
        .into();
    assert!(expression.execute(Handoff::Run).is_err());
    assert!(
        !Process::new("sh")
            .args(["-c", &expression.shell(Handoff::Run).unwrap()])
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[cfg(unix)]
#[test]
fn shell_rejects_non_utf8_instead_of_changing_the_value() {
    use std::os::unix::ffi::OsStringExt;
    let word = Word::new(std::ffi::OsString::from_vec(vec![0xff])).unwrap();
    assert!(word.shell().is_err());
}
