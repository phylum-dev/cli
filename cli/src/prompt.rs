use ansi_term::Color::White;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};

/// Prompt the user for the threshold value and action associated with a given
/// threshold.
pub fn prompt_threshold(name: &str) -> Result<(i32, &str), std::io::Error> {
    let threshold = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "{} Threshold",
            format_args!("{}", White.paint(name.to_uppercase()))
        ))
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.chars().all(char::is_numeric) {
                let val = input.parse::<i32>().unwrap();
                if (0..=100).contains(&val) {
                    Ok(())
                } else {
                    Err("Make sure to specify a number between 0-100")
                }
            } else {
                Err("Threshold must be a number between 0-100")
            }
        })
        .report(true)
        .interact_text()?;

    if threshold == "0" {
        println!(
            "\nDisabling {} risk domain",
            format_args!("{}", White.paint(name))
        );
        println!("\n-----\n");
        return Ok((0, "none"));
    }

    println!(
        "\nWhat should happen if a score falls below the {} threshold?\n",
        format_args!("{}", White.paint(name))
    );

    let items = vec![
        "Break the CI/CD build",
        "Print a warning message",
        "Do nothing",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .report(true)
        .interact()
        .unwrap();
    let action = items[selection];
    println!("✔ {} Action · {}", White.paint(name.to_uppercase()), action);
    println!("\n-----\n");

    Ok((
        threshold.parse::<i32>().unwrap(),
        match selection {
            // Convert the provided selection index into a string suitable for sending
            // back to the API endpoint responsible for handling user settings.
            0 => "break",
            1 => "warn",
            2 => "none",
            _ => "warn", // We shouldn't be able to make it here.
        },
    ))
}
