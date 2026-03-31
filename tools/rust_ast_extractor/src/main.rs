use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use syn::visit::Visit;
use syn::{ExprMethodCall, ExprPath, FnArg, ImplItem, ImplItemFn, Item, ItemImpl, Pat, Local, Macro};

#[derive(Serialize)]
struct MonsterInfo {
    file_path: String,
    has_roll_move: bool,
    has_roll_move_custom: bool,
    sig_has_num: bool,
    uses_num: bool,
    declares_num: bool,
    calls_random99: bool,
}

#[derive(Default)]
struct FnVisitor {
    uses_num: bool,
    declares_num: bool,
    calls_random99: bool,
}

impl<'ast> Visit<'ast> for FnVisitor {
    fn visit_local(&mut self, i: &'ast Local) {
        if let Pat::Ident(pat_ident) = &i.pat {
            if pat_ident.ident == "num" {
                self.declares_num = true;
            }
        }
        // Also check if `mut num` is declared
        if let Pat::Type(pat_type) = &i.pat {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                if pat_ident.ident == "num" {
                    self.declares_num = true;
                }
            }
        }
        syn::visit::visit_local(self, i);
    }

    fn visit_expr_path(&mut self, i: &'ast ExprPath) {
        if let Some(ident) = i.path.get_ident() {
            if ident == "num" {
                self.uses_num = true;
            }
        }
        syn::visit::visit_expr_path(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();
        if method_name == "random" {
            if let Some(arg) = i.args.first() {
                if let syn::Expr::Lit(expr_lit) = arg {
                    if let syn::Lit::Int(lit_int) = &expr_lit.lit {
                        if lit_int.base10_parse::<i32>().unwrap_or(0) == 99 {
                            self.calls_random99 = true;
                        }
                    }
                }
            }
        } else if method_name == "random_range" {
            let mut is_99 = false;
            for arg in &i.args {
                if let syn::Expr::Lit(expr_lit) = arg {
                    if let syn::Lit::Int(lit_int) = &expr_lit.lit {
                        if lit_int.base10_parse::<i32>().unwrap_or(0) == 99 {
                            is_99 = true;
                        }
                    }
                }
            }
            if is_99 {
                self.calls_random99 = true;
            }
        }
        syn::visit::visit_expr_method_call(self, i);
    }
}

fn main() {
    let base_dir = Path::new("../../src/content/monsters");
    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(base_dir) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| ext == "rs") {
            let path_str = entry.path().to_string_lossy().to_string();
            // skip mod.rs
            if path_str.ends_with("mod.rs") || path_str.contains("factory.rs") || path_str.contains("encounter_pool") {
                continue;
            }

            let mut info = MonsterInfo {
                file_path: entry.path().canonicalize().unwrap().to_string_lossy().to_string(),
                has_roll_move: false,
                has_roll_move_custom: false,
                sig_has_num: false,
                uses_num: false,
                declares_num: false,
                calls_random99: false,
            };

            let content = fs::read_to_string(entry.path()).unwrap();
            let parsed_file = syn::parse_file(&content).unwrap_or_else(|_| panic!("Failed to parse: {}", path_str));

            for item in &parsed_file.items {
                match item {
                    Item::Impl(item_impl) => {
                        // Check if it's `impl MonsterBehavior for X`
                        let is_monster_behavior = if let Some((_, path, _)) = &item_impl.trait_ {
                            path.segments.last().map(|s| s.ident.to_string()) == Some("MonsterBehavior".to_string())
                        } else {
                            false
                        };

                        for impl_item in &item_impl.items {
                            if let ImplItem::Fn(func) = impl_item {
                                let func_name = func.sig.ident.to_string();
                                
                                if (is_monster_behavior && func_name == "roll_move") || func_name == "roll_move_custom" {
                                    if func_name == "roll_move" {
                                        info.has_roll_move = true;
                                    } else {
                                        info.has_roll_move_custom = true;
                                    }

                                    // Check signature
                                    for input in &func.sig.inputs {
                                        if let FnArg::Typed(pat_type) = input {
                                            // Handling both `num: i32` and `mut num: i32`
                                            let mut ident_name = None;
                                            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                                                ident_name = Some(pat_ident.ident.to_string());
                                            } else if let Pat::Type(t2) = &*pat_type.pat {
                                                if let Pat::Ident(pat_ident) = &*t2.pat {
                                                    ident_name = Some(pat_ident.ident.to_string());
                                                }
                                            }
                                            
                                            // Syn AST handles mut in PatIdent
                                            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                                                let name = pat_ident.ident.to_string();
                                                if name == "num" || name == "_num" {
                                                    info.sig_has_num = true;
                                                }
                                            }
                                        }
                                    }

                                    // Traverse body
                                    let mut visitor = FnVisitor::default();
                                    visitor.visit_impl_item_fn(func);
                                    info.uses_num = visitor.uses_num;
                                    info.declares_num = visitor.declares_num;
                                    info.calls_random99 = visitor.calls_random99;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            results.push(info);
        }
    }

    let json = serde_json::to_string_pretty(&results).unwrap();
    fs::write("../../tools/rust_monsters.json", json).unwrap();
    println!("Extracted {} monsters to rust_monsters.json", results.len());
}
