#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::{eval_run, eval_vm_bundled};

fn program(path: &std::path::Path) -> String {
    format!(
        "import \"intolang://builtin/std\";

pub Reader() {{
    pub fun read(path: Path): Binary {{
        return Binary.read(path);
    }}
}}

pub Decoder() {{
    pub fun open(data: Binary): Int64 {{
        return data.length();
    }}

    pub fun load(path: Path): Int64 {{
        now into reader: Reader = Reader();
        late into data: Binary = reader.read(path);
        now into size = this.open(data.move());
        return size;
    }}
}}

init {{
    now into decoder = Decoder();
    println(decoder.load(\"{}\"));
    now into binary = Binary.read(\"{}\");
    println(binary.at(0));
    println(binary.text());
}}
",
        path.display(),
        path.display()
    )
}

#[test]
fn a_file_reads_through_a_function_that_takes_a_path() {
    let source = "import \"intolang://builtin/std\";
import \"fs://file\";

pub Holder() {
    pub fun read(path: Path): Int64 {
        now into file: File = File();
        late into data: Binary = file.read(path);
        return data.length();
    }
}

now into holder = Holder();
now into size = holder.read(\"clip.mp4\");
";
    let packages = compiler::builtin_packages_dir();
    let message = backend::lower_with_packages(source, Some(&packages)).unwrap();
    assert!(message.contains("into_binary_read"), "{message}");
}

#[test]
fn the_bundled_packages_pass_a_path_to_binary() {
    let source = "import \"intolang://builtin/std\";
import \"fs://file\";
import \"ffmpeg://decoder\";

now into ffmpeg = FFmpeg();
now into video = ffmpeg.read(\"clip.mp4\");
now into file = File();
now into bytes = file.read(\"clip.mp4\");
";
    let packages = compiler::builtin_packages_dir();
    let message = backend::lower_with_packages(source, Some(&packages)).unwrap();
    assert!(message.contains("into_binary_read"), "{message}");
}

#[test]
fn a_path_reaches_binary_through_a_late_binding() {
    let directory = std::env::temp_dir().join(format!("into-binary-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("data.bin");
    std::fs::write(&path, "Into").unwrap();
    let source = program(&path);
    let expected = "4\n73\nInto\n";
    let virtual_run = eval_vm_bundled(&source, VmOptions::default());
    let compiled = eval_run(&source);
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(virtual_run.unwrap().stdout, expected);
    let compiled = compiled.unwrap();
    assert!(compiled.success, "{}", compiled.stderr);
    assert_eq!(compiled.stdout, expected);
}
