#[cfg(test)]
mod test {
    use crate::log_parser::LogParser;
    use crate::log_parser_tree_builder::show;
    use crate::log_render::LogRender;
    use crate::log_render_color::ColorProfile;
    use crate::log_transfomer_into_json::LogTransfomer;
    use std::fs;
    use std::fs::File;
    use std::io::{BufWriter, Write};

    #[test]
    fn test_large_file() {
        let data = fs::read("./src/testdata/large.bin").unwrap();
        let mut rdata = data.as_slice();

        let file = File::create("./src/testdata/large.jsonl").unwrap();
        // По умолчанию буфер 8 КБ, но для 2 ГБ мяса можно бахнуть и 128 КБ
        let mut writer = BufWriter::with_capacity(512 * 1024, file);

        let mut dst: Vec<u8> = Vec::with_capacity(512 * 1024);
        // let mut parser = LogParser::new();

        let mut render = LogTransfomer::new();

        while rdata.len() > 0 {
            dst.clear();
            unsafe {
                rdata = render.transform_json(&mut dst, rdata).unwrap();
                // let (_, x) =  parser.parse_log_data(rdata).unwrap();
                // rdata = x;
            }
            dst.push(b'\n');
            writer.write_all(&dst).unwrap();
        }
    }

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
            show_json_output(file);
        }
    }

    fn show_json_output(file: &str) {
        let data = fs::read(file).unwrap();
        let rdata = data.as_slice();

        unsafe {
            let mut dst: Vec<u8> = Vec::new();
            let mut render = LogTransfomer::new();
            render.transform_json(&mut dst, rdata).unwrap();
            dst.push(b'\n');

            println!("binary[{} bytes] json[{} bytes]", rdata.len(), dst.len());
            print!("{}", String::from_utf8_lossy(&dst));
        }
    }

    fn show_file_output(file: &str) {
        let data = fs::read(file).unwrap();
        let rdata = data.as_slice();

        let mut parser = LogParser::new();

        unsafe {
            let mut render = LogRender::new(ColorProfile::light());
            let (record, _) = parser.parse_log_data(rdata).unwrap();
            parser.make_record(&mut render);

            let mut dst: Vec<u8> = Vec::new();
            (&mut render).render(&mut dst, record);

            print!("{}", String::from_utf8_lossy(&dst));
        }
    }
}
