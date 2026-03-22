#[cfg(test)]
mod test {
    use crate::log_parser::LogParser;
    use crate::log_render::LogRender;
    use std::fs;
    use crate::log_parser_tree_builder::show;
    use crate::log_render_color::ColorProfile;

    #[test]
    fn showcase_for_log_parser_and_render() {
        let files = &[
            &"./src/testdata/message_only.bin",
            &"./src/testdata/message_short_flat_context.bin",
            &"./src/testdata/message_with_binary_in_ctx.bin",
            &"./src/testdata/message_with_loads_of_slices.bin",
            &"./src/testdata/group.bin",
            &"./src/testdata/errors.bin",
            &"./src/testdata/error_intmixed.bin",
            &"./src/testdata/error_foreign_root.bin",
            &"./src/testdata/panic.bin",
        ];

        for file in files {
            show_file_output(file);
        }
    }

    fn show_file_output(file: &str) {
        let data = fs::read(file).unwrap();
        let rdata = data.as_slice();

        let mut parser = LogParser::new();

        unsafe {
            let mut render = LogRender::new(ColorProfile::dark());
            let (record, _)=parser.parse_log_data(rdata).unwrap();
            parser.make_record(&mut render);

            let mut dst: Vec<u8> = Vec::new();
            (&mut render).render(&mut dst, record);

            print!("{}", String::from_utf8_lossy(&dst));
        }
    }
}
