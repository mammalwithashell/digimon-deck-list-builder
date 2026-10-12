//! Read-only native inventory; output is always incomplete reference evidence.
#[path="implementation_inventory/core.rs"] mod core;
use std::{env,fs,path::Path};
fn collect(root:&Path,dir:&Path,files:&mut Vec<core::FileInput>)->Result<(),String>{
 for entry in fs::read_dir(dir).map_err(|e|e.to_string())? {
  let entry=entry.map_err(|e|e.to_string())?;let kind=entry.file_type().map_err(|e|e.to_string())?;
  if kind.is_symlink(){return Err("symlink in implementation root is unsupported".into());}
  let path=entry.path();
  if kind.is_dir(){collect(root,&path,files)?;}
  else if matches!(path.extension().and_then(|s|s.to_str()),Some("yaml"|"yml")) {
   let relative=path.strip_prefix(root).map_err(|e|e.to_string())?.to_str().ok_or("non-UTF8 path")?.replace('\\',"/");
   files.push(core::FileInput{path:relative,bytes:fs::read(path).map_err(|e|e.to_string())?});
  }
 }
 Ok(())
}
fn run()->Result<(),String>{
 let a:Vec<_>=env::args_os().skip(1).collect();if a.len()!=3{return Err("usage: implementation_inventory <E-array.json> <YAML-root> <new-output.json>".into());}
 let ids:Vec<String>=serde_json::from_slice(&fs::read(&a[0]).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 let root=Path::new(&a[1]);if fs::symlink_metadata(root).map_err(|e|e.to_string())?.file_type().is_symlink(){return Err("symlink root unsupported".into());}
 let mut files=Vec::new();collect(root,root,&mut files)?;
 let result=core::build_inventory(ids,files)?;
 let output=fs::OpenOptions::new().write(true).create_new(true).open(&a[2]).map_err(|e|e.to_string())?;
 serde_json::to_writer(output,&result).map_err(|e|e.to_string())?;Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1)}}
