// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/cli/fmt_errors.rs

//! This mod provides DenoError to unify errors across Deno.

use std::fmt::Write as _;

use deno_runtime::colors::{cyan, italic_bold, red, yellow};
use deno_runtime::deno_core::error::{format_file_name, JsError, JsStackFrame};

// Keep in sync with `/core/error.js`.
pub fn format_location(frame: &JsStackFrame) -> String {
    let _internal = frame.file_name.as_ref().map_or(false, |f| f.starts_with("deno:"));
    if frame.is_native {
        return cyan("native").to_string();
    }
    let mut result = String::new();
    let file_name = frame.file_name.clone().unwrap_or_default();
    if !file_name.is_empty() {
        result += &cyan(&format_file_name(&file_name)).to_string();
    } else {
        if frame.is_eval {
            result += &(cyan(&frame.eval_origin.as_ref().unwrap()).to_string() + ", ");
        }
        result += &cyan("<anonymous>").to_string();
    }
    if let Some(line_number) = frame.line_number {
        write!(result, ":{}", yellow(&line_number.to_string())).unwrap();
        if let Some(column_number) = frame.column_number {
            write!(result, ":{}", yellow(&column_number.to_string())).unwrap();
        }
    }
    result
}

fn format_frame(frame: &JsStackFrame) -> String {
    let _internal = frame.file_name.as_ref().map_or(false, |f| f.starts_with("deno:"));
    let is_method_call = !(frame.is_top_level.unwrap_or_default() || frame.is_constructor);
    let mut result = String::new();
    if frame.is_async {
        result += "async ";
    }
    if frame.is_promise_all {
        result += &italic_bold(&format!(
            "Promise.all (index {})",
            frame.promise_index.unwrap_or_default()
        ))
        .to_string();
        return result;
    }
    if is_method_call {
        let mut formatted_method = String::new();
        if let Some(function_name) = &frame.function_name {
            if let Some(type_name) = &frame.type_name {
                if !function_name.starts_with(type_name) {
                    write!(formatted_method, "{}.", type_name).unwrap();
                }
            }
            formatted_method += function_name;
            if let Some(method_name) = &frame.method_name {
                if !function_name.ends_with(method_name) {
                    write!(formatted_method, " [as {}]", method_name).unwrap();
                }
            }
        } else {
            if let Some(type_name) = &frame.type_name {
                write!(formatted_method, "{}.", type_name).unwrap();
            }
            if let Some(method_name) = &frame.method_name {
                formatted_method += method_name
            } else {
                formatted_method += "<anonymous>";
            }
        }
        result += &italic_bold(&formatted_method).to_string();
    } else if frame.is_constructor {
        result += "new ";
        if let Some(function_name) = &frame.function_name {
            write!(result, "{}", italic_bold(&function_name)).unwrap();
        } else {
            result += &cyan("<anonymous>").to_string();
        }
    } else if let Some(function_name) = &frame.function_name {
        result += &italic_bold(&function_name).to_string();
    } else {
        result += &format_location(frame);
        return result;
    }
    write!(result, " ({})", format_location(frame)).unwrap();
    result
}

/// Take an optional source line and associated information to format it into
/// a pretty printed version of that line.
fn format_maybe_source_line(
    source_line: Option<&str>,
    column_number: Option<i64>,
    is_error: bool,
    level: usize,
) -> String {
    if source_line.is_none() || column_number.is_none() {
        return "".to_string();
    }

    let source_line = source_line.unwrap();
    // sometimes source_line gets set with an empty string, which then outputs
    // an empty source line when displayed, so need just short circuit here.
    if source_line.is_empty() {
        return "".to_string();
    }
    if source_line.contains("Couldn't format source line: ") {
        return format!("\n{}", source_line);
    }

    let mut s = String::new();
    let column_number = column_number.unwrap();

    if column_number as usize > source_line.len() {
        return format!(
            "\n{} Couldn't format source line: Column {} is out of bounds (source may have \
             changed at runtime)",
            yellow("Warning"),
            column_number,
        );
    }

    for _i in 0..(column_number - 1) {
        if source_line.chars().nth(_i as usize).unwrap() == '\t' {
            s.push('\t');
        } else {
            s.push(' ');
        }
    }
    s.push('^');
    let color_underline = if is_error { red(&s).to_string() } else { cyan(&s).to_string() };

    let indent = format!("{:indent$}", "", indent = level);

    format!("\n{}{}\n{}{}", indent, source_line, indent, color_underline)
}

fn format_js_error_inner(js_error: &JsError, is_child: bool) -> String {
    let mut s = String::new();
    s.push_str(&js_error.exception_message);
    if let Some(aggregated) = &js_error.aggregated {
        for aggregated_error in aggregated {
            let error_string = format_js_error_inner(aggregated_error, true);
            for line in error_string.trim_start_matches("Uncaught ").lines() {
                write!(s, "\n    {}", line).unwrap();
            }
        }
    }
    let column_number = js_error
        .source_line_frame_index
        .and_then(|i| js_error.frames.get(i).unwrap().column_number);
    s.push_str(&format_maybe_source_line(
        if is_child { None } else { js_error.source_line.as_deref() },
        column_number,
        true,
        0,
    ));
    for frame in &js_error.frames {
        write!(s, "\n    at {}", format_frame(frame)).unwrap();
    }
    if let Some(cause) = &js_error.cause {
        let error_string = format_js_error_inner(cause, true);
        write!(s, "\nCaused by: {}", error_string.trim_start_matches("Uncaught ")).unwrap();
    }
    s
}

pub fn format_js_error(js_error: &JsError) -> String {
    format_js_error_inner(js_error, false)
}
