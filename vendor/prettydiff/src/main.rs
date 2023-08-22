use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use structopt::StructOpt;

/// Side-by-side diff for two files
#[derive(StructOpt, Debug)]
#[structopt(name = "prettydiff")]
struct Opt {
    /// Left file
    #[structopt(name = "LEFT", parse(from_os_str))]
    left: PathBuf,
    /// Right file
    #[structopt(name = "RIGHT", parse(from_os_str))]
    right: PathBuf,
    /// Don't show lines numbers
    #[structopt(long = "disable_lines")]
    disable_lines: bool,
    /// Show non-changed blocks
    #[structopt(long = "show_same")]
    show_same: bool,
    /// Align new lines inside change block
    #[structopt(long = "disable_align")]
    disable_align: bool,
}

fn read_file(path: &PathBuf) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();

    let left_data = read_file(&opt.left)?;
    let left_name = opt.left.into_os_string().into_string().unwrap();

    let right_data = read_file(&opt.right)?;
    let right_name = opt.right.into_os_string().into_string().unwrap();

    prettydiff::diff_lines(&left_data, &right_data)
        .names(&left_name, &right_name)
        .set_show_lines(!opt.disable_lines)
        .set_diff_only(!opt.show_same)
        .set_align_new_lines(!opt.disable_align)
        .prettytable();

    Ok(())
}
