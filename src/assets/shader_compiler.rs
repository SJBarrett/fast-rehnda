use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use glob::glob;
use shaderc::{CompileOptions, Compiler, ShaderKind};

pub fn compile_all_files() {
    let files_to_compile = files_to_compile();
    let compiler = Compiler::new().expect("Failed to build compiler");
    files_to_compile.iter().for_each(|to_compile| compile_to_spirv(&compiler, to_compile));
}

fn compile_to_spirv(compiler: &Compiler, to_compile: &ToCompile) {
    let file_path = to_compile.path_buf.as_path();
    let mut file = File::open(to_compile.path_buf.as_path()).unwrap();
    let mut file_data = String::new();
    file.read_to_string(&mut file_data).unwrap();
    let mut compile_options = CompileOptions::new().unwrap();
    compile_options.set_generate_debug_info();
    let binary_result = compiler.compile_into_spirv(
        file_data.as_str(),
        to_compile.kind,
        file_path.file_name().unwrap().to_str().unwrap(),
        "main",
        Some(&compile_options),
    ).unwrap();
    let out_file_name = format!("shaders/spirv/{}_spv", to_compile.path_buf.file_name().unwrap().to_str().unwrap());
    let mut out_file = File::create(out_file_name).unwrap();
    out_file.write_all(binary_result.as_binary_u8()).unwrap();
}

fn files_to_compile() -> Vec<ToCompile> {
    let mut to_compiles: Vec<ToCompile> = Vec::new();
    for entry in glob("shaders/src/**/*").unwrap() {
        let a = entry.unwrap();
        let extension = a.extension().unwrap();
        match extension.to_str().unwrap() {
            "vert" => to_compiles.push(ToCompile {
                path_buf: a,
                kind: ShaderKind::Vertex,
            }),
            "frag" => to_compiles.push(ToCompile {
                path_buf: a,
                kind: ShaderKind::Fragment,
            }),
            _ => panic!("Unsupported extension in shaders")
        }
    }
    to_compiles
}

struct ToCompile {
    path_buf: PathBuf,
    kind: ShaderKind,
}