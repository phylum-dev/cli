use std::fmt::Write;
use std::string::ToString;

use clap::{Arg, ArgAction, Command};

/// Markdown formatter.
pub struct Generator<'a> {
    cmd: &'a mut Command,
}

impl<'a> Generator<'a> {
    pub fn new(cmd: &'a mut Command) -> Self {
        cmd.build();
        Self { cmd }
    }

    /// Generate the markdown for this command.
    pub fn generate(self) -> Vec<Markdown> {
        self.generate_prefixed(Vec::new())
    }

    /// Generate subcommand markdown with a list of parent commands.
    fn generate_prefixed(self, parents: Vec<String>) -> Vec<Markdown> {
        // Add the markdown for the command itself.
        let mut pages = vec![Markdown::from_command(self.cmd, parents)];

        // Generate subcommand markdown recursively.
        for cmd in self.cmd.get_subcommands_mut().filter(|cmd| !cmd.is_hide_set()) {
            // Ignore `help` subcommands.
            if cmd.get_name() == "help" {
                continue;
            }

            let parents = pages[0].command.clone();
            pages.append(&mut Generator::new(cmd).generate_prefixed(parents));
        }

        pages
    }
}

/// Generated markdown file.
pub struct Markdown {
    /// Name of the (sub)command and all its parents.
    ///
    /// The order of commands goes from topmost as the first element to the
    /// command itself in the last position.
    pub command: Vec<String>,
    /// Generated markdown page.
    pub content: String,
}

impl Markdown {
    fn from_command(cmd: &mut Command, mut parents: Vec<String>) -> Self {
        // Add (sub)command to command list.
        parents.push(cmd.get_name().into());

        let mut markdown = String::new();

        // Add command description.
        if let Some(description) =
            cmd.get_long_about().or_else(|| cmd.get_about()).map(ToString::to_string)
        {
            let description = escape_markdown(&description);
            let _ = writeln!(markdown, "{description}");
        }

        // Add usage example.
        let usage = cmd.render_usage().to_string();
        let _ = writeln!(markdown, "\n```sh\n{usage}\n```");

        // Add positional arguments.
        let mut positionals = cmd.get_positionals().flat_map(generate_argument).peekable();
        if positionals.peek().is_some() {
            let _ = writeln!(markdown, "\n### Arguments");
        }
        for positional in positionals {
            let _ = writeln!(markdown, "\n{positional}");
        }

        // Add options.
        let mut options = cmd
            .get_arguments()
            .filter(|arg| !arg.is_positional())
            .flat_map(generate_argument)
            .peekable();
        if options.peek().is_some() {
            let _ = writeln!(markdown, "\n### Options");
        }
        for option in options {
            let _ = writeln!(markdown, "\n{option}");
        }

        // Add subcommands.
        let mut subcommands = cmd
            .get_subcommands()
            .filter(|cmd| !cmd.is_hide_set() && cmd.get_name() != "help")
            .peekable();
        if subcommands.peek().is_some() {
            let _ = writeln!(markdown, "\n### Commands\n");
        }
        for cmd in subcommands {
            let name = cmd.get_name();
            let human_path = format!("{} {name}", parents.join(" "));
            let link_path = format!("{}_{name}", parents.join("_"));
            let _ = writeln!(markdown, "* [{}](./{})", human_path, link_path);
        }

        Self { command: parents, content: markdown }
    }
}

/// Convert argument to markdown.
fn generate_argument(arg: &Arg) -> Option<String> {
    // Don't show hidden arguments.
    if arg.is_hide_set() {
        return None;
    }

    let mut markdown = String::new();

    // Add short option.
    if let Some(short) = arg.get_short() {
        let _ = write!(markdown, "-{short}");
    }

    // Add long option.
    if let Some(long) = arg.get_long() {
        if !markdown.is_empty() {
            markdown += ", ";
        }
        let _ = write!(markdown, "--{long}");
    }

    // Add arguments.
    let min_required = arg.get_num_args().map_or(0, |num| num.min_values());
    let all_optional = arg.is_positional() && !arg.is_required_set();
    if let Some(value_names) = arg.get_value_names() {
        if !markdown.is_empty() {
            markdown += " ";
        }

        let delimiter = arg.get_value_delimiter().unwrap_or(' ');

        for (i, value_name) in value_names.iter().enumerate() {
            // Add separator between parameters.
            if i != 0 {
                markdown.push(delimiter);
            }

            if i >= min_required || all_optional {
                let _ = write!(markdown, "[{value_name}]");
            } else {
                let _ = write!(markdown, "<{value_name}>");
            }
        }
    }

    // Add repetition indicator.
    if matches!(arg.get_action(), ArgAction::Count) {
        let _ = write!(markdown, "...");
    }

    // Add description.
    let description = arg.get_long_help().or_else(|| arg.get_help()).map(ToString::to_string);
    if let Some(description) = description {
        let description = escape_markdown(&description);
        let _ = write!(markdown, "\n&emsp; {description}");
    }

    // Add accepted values.
    let possible_values = arg.get_possible_values();
    let mut possible_values = possible_values.iter().filter(|value| !value.is_hide_set());
    if let Some(possible_value) = possible_values.next() {
        let _ = write!(markdown, "\n&emsp; Accepted values: `{}`", possible_value.get_name());
    }
    for possible_value in possible_values {
        let _ = write!(markdown, ", `{}`", possible_value.get_name());
    }

    Some(markdown)
}

/// Escape markdown for proper formatting.
fn escape_markdown(description: &str) -> String {
    let mut output = String::with_capacity(description.len());

    // Escape leading whitespace with `&nbsp;`.
    for line in description.lines() {
        let start_trimmed = line.trim_start_matches(' ');
        output += &"&nbsp;".repeat(line.len() - start_trimmed.len());
        output += start_trimmed;
        output += "\n";
    }
    output.pop();

    // Escape ` with \`.
    output = output.replace('`', "\\`");

    // Escape # with \#.
    output = output.replace('#', "\\#");

    output
}
