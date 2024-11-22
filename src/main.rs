use std::env::args;
use std::process::{exit, Child, Command, Stdio};
use console::{style, Style};
use std::{path::Path, time::Duration};
use std::thread::sleep;
use notify::{self, RecursiveMode};
use notify_debouncer_mini::{new_debouncer_opt, Config};


fn description() {
    println!("{} {}",
             style("->").bold().green(),
             style("dirwatch is a simple CLI tool that watches a directory\n\
                       and restart the command passed every time a change is detected.").bold().blue())
}

fn usage() {
    let title_style = Style::new().bold().green();
    let text_style = Style::new().bold().cyan();
    println!("{}\n\
                {}\n\
              {}\n\
                {:18}{}\n\
                {:18}{}\n\
              {}\n\
                {}\n\
                {}",
             title_style.apply_to("Usage:"),
             text_style.apply_to("\tdirwatch [OPTION]"),
             title_style.apply_to("Options:"),
             text_style.apply_to("\t-h, --help"), text_style.apply_to("Prints help information"),
             text_style.apply_to("\t-v, --version"), text_style.apply_to("Prints version information"),
             title_style.apply_to("Conditions:"),
             text_style.apply_to("\t- you need to pass it a valid command."),
             text_style.apply_to("\t- don't launch it in a directory with too much files in it, it might overload your PC."));
}

fn version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    exit(0)
}

fn error(err: &str) {
    println!("{} {}", style("Error:").bold().red(), style(err).red());
    println!();
    usage();
    exit(1);
}

fn help() {
    description();
    println!();
    usage();
    exit(0);
}


fn main() {
    ctrlc::set_handler(move || {
        println!();
        println!("{}", style("Exiting program...").bold().yellow());
        exit(0);
    }).expect("Error setting SIGINT handler");

    let args = args().skip(1).collect::<Vec<_>>();
    if !args.is_empty() {
        match args.first().unwrap().as_str() {
            "-v" | "--version" => version(),
            "-h" | "--help" => help(),
            _ => {}
        }
    }

    watch(args)
}

fn watch(cmds: Vec<String>) {
    // run the command and retrieve the ChildCommand
    let mut child_command = ChildCommand::new(cmds);
    child_command.start();

    // setup debouncer
    let (tx, rx) = std::sync::mpsc::channel();

    // notify backend configuration
    let backend_config = notify::Config::default().with_poll_interval(Duration::from_secs(1));

    // debouncer configuration
    let debouncer_config = Config::default()
        .with_timeout(Duration::from_millis(1000))
        .with_notify_config(backend_config);

    // select backend via fish operator, here PollWatcher backend
    let mut debouncer = new_debouncer_opt::<_, notify::PollWatcher>(debouncer_config, tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new("."), RecursiveMode::Recursive)
        .unwrap();

    // print all events, non-returning
    for result in rx {
        match result {
            Ok(_) => {
                child_command.stop();
                child_command.start()
            },
            Err(err) => error(err.to_string().as_str()),
        }
    }
}

struct ChildCommand {
    cmds: Vec<String>,
    process: Option<Child>,
}

impl ChildCommand {
    fn new(cmds: Vec<String>) -> ChildCommand {
        ChildCommand {
            cmds,
            process: None,
        }
    }

    fn start(&mut self) {
        // Printing the command
        print!("{} ", style("Running command:").bold().green());
        for cmd in &self.cmds {
            print!("{} ", cmd)
        }
        println!();

        let cmds = self.cmds.clone();
        match &mut self.process {
            None => {}
            Some(p) => {
                p.kill().unwrap();
                p.wait().unwrap();
            }
        }
        self.process = Some(Command::new(cmds.first().unwrap())
                .args( & cmds[1..])
                .stdin(Stdio::piped())
                .spawn().unwrap());
        sleep(Duration::from_millis(100))
    }

    fn stop(&mut self) {
        match &mut self.process {
            None => {}
            Some(p) => {
                p.kill().unwrap();

                // Wait and sleep a bit to properly kill the process before restarting it
                p.wait().unwrap();
                sleep(Duration::from_millis(500));
            }
        }
    }
}
