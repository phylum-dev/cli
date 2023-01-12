use console::style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};
use phylum_types::types::user_settings::Threshold;

/// Project thresholds which cannot be disabled.
const ALWAYS_ENABLED_THRESHOLDS: [&str; 1] = ["total project"];

/// Prompt the user for the threshold value and action associated with a given
/// threshold.
pub fn prompt_threshold(name: &str) -> Result<Threshold, std::io::Error> {
    let threshold = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "{} Threshold",
            format_args!("{}", style(name.to_uppercase()).white())
        ))
        .validate_with(|input: &String| -> Result<(), String> {
            if input.eq_ignore_ascii_case("disabled") {
                if ALWAYS_ENABLED_THRESHOLDS.contains(&name) {
                    Err(format!("Cannot disable {name} threshold"))
                } else {
                    Ok(())
                }
            } else if input.chars().all(char::is_numeric) {
                let val = input.parse::<i32>().unwrap();
                if (0..=100).contains(&val) {
                    Ok(())
                } else {
                    Err("Make sure to specify a number between 0-100".into())
                }
            } else {
                Err("Threshold must be a number between 0-100 or 'Disabled'".into())
            }
        })
        .interact_text()?;

    if threshold.eq_ignore_ascii_case("disabled") {
        println!("\nDisabling {} risk domain", format_args!("{}", style(name).white()));
        println!("\n-----\n");

        return Ok(Threshold { action: "none".into(), threshold: 0., active: false });
    }

    println!(
        "\nWhat should happen if a score falls below the {} threshold?\n",
        format_args!("{}", style(name).white())
    );

    let items = vec!["Break the CI/CD build", "Print a warning message", "Do nothing"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .report(true)
        .interact()
        .unwrap();
    let action = items[selection];
    println!("✔ {} Action · {}", style(name.to_uppercase()).white(), action);
    println!("\n-----\n");

    let threshold = threshold.parse::<i32>().unwrap() as f32 / 100.;

    let action = match selection {
        // Convert the provided selection index into a string suitable for sending
        // back to the API endpoint responsible for handling user settings.
        0 => "break",
        1 => "warn",
        2 => "none",
        _ => "warn", // We shouldn't be able to make it here.
    }
    .to_owned();

    Ok(Threshold { active: true, threshold, action })
}
