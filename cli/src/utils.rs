use prettytable::format;

pub fn table_format(left_pad: usize, right_pad: usize) -> format::TableFormat {
    format::FormatBuilder::new()
        .column_separator(' ')
        .borders(' ')
        .separators(
            &[format::LinePosition::Top, format::LinePosition::Bottom],
            format::LineSeparator::new(' ', ' ', ' ', ' '),
        )
        .padding(left_pad, right_pad)
        .build()
}
