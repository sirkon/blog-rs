use super::*;

#[cfg(test)]
mod test {
    use crate::log_parser::LogParser;
    use crate::log_render::LogRender;
    use std::fs;

    #[test]
    fn test_message_only() {
        let data = fs::read(&"./src/testdata/message_only.bin").unwrap();
        let rdata = data.as_slice();

        let mut parser = LogParser::new();

        unsafe {
            let mut render = LogRender::new();
            parser.parse_log_data(rdata).unwrap();
            parser.make_record(&mut render);

            let mut dst: Vec<u8> = Vec::new();
            (&mut render).render(&mut dst, &rdata[parser.source_off..]);

            println!("{}", String::from_utf8_lossy(&dst));
        }
    }

    #[test]
    fn test_message_with_short_flat_context() {
        let data = fs::read(&"./src/testdata/message_short_flat_context.bin").unwrap();
        let rdata = data.as_slice();

        let mut parser = LogParser::new();

        unsafe {
            let mut render = LogRender::new();
            parser.parse_log_data(rdata).unwrap();
            parser.make_record(&mut render);

            let mut dst: Vec<u8> = Vec::new();
            (&mut render).render(&mut dst, &rdata[parser.source_off..]);

            println!("{}", String::from_utf8_lossy(&dst));
        }
    }
}
