
fn main() {

}


#[cfg(test)]
mod test{
    use std::io::{self, Write};

    #[test]
    fn buf_writer() {
        let stdout = io::stdout();
        let mut handle = io::BufWriter::new(stdout);
        writeln!(handle, "foo: {}", 42);
    }

    #[test]
    fn progress_bar() {
        let pb = indicatif::ProgressBar::new(100);
        for i in 0..100 {
            do_hard_work();
            pb.println(format!("[+] finished #{}", i));
            pb.inc(1);
        }
        pb.finish_with_message("done");
    }
}