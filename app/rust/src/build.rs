use std::collections::HashMap;
#[allow(unused)]
use std::{fs::File, io::Write, path::Path};
use std::fs::{self, OpenOptions};
use std::time::{SystemTime, UNIX_EPOCH};
#[allow(unused)]
use dbml_rs;
#[allow(unused)]
use tap::prelude::*;
#[allow(unused)]
use itertools::*;

fn debug_println(s: &String) {
    let out_path = "/tmp/db_types.log";
    let out_file = Path::new(out_path);

    let mut mut_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(out_file)
        .expect(&format!("Failed to open {}", out_path));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simple time breakdown (not timezone-aware, just for logs)
    let seconds_in_day = now % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;
    let seconds = seconds_in_day % 60;

    let timestamp = format!("[{:02}:{:02}:{:02}]", hours, minutes, seconds);

    writeln!(mut_file, "{} {}", timestamp, s).unwrap();
}

fn begin_comment_section(name: &str) -> String {
    format!(
        "// --------------------------------------------------------------------------------------------\n\
         // \\begin{{Table {name}}}\n\n"
    )
}

fn end_comment_section(name: &str) -> String {
    format!(
        "// \\end{{Table {name}}}\n\
         // --------------------------------------------------------------------------------------------\n\n"
    )
}

fn to_snake_case(s: &str) -> String {
    s
        .chars()
        .enumerate()
        .map(|(i, c)| match (i, c.is_uppercase()) {
            (0, true) => {
                format!("{}", c.to_lowercase())
            }
            (0, false) => {
                format!("{c}")
            }
            _ => {
                if c.is_uppercase() {
                    format!("_{}", c.to_lowercase())
                } else {
                    format!("{c}")
                }
            }
        })
        .join("")
}

fn get_col_name_or_ty_to_ty(
    cat_ty_name: &str, 
    col_name_or_ty: &str, 
    do_concat_table_name: bool,
) -> String {
    if do_concat_table_name {
        format!("{cat_ty_name}{col_name_or_ty}")
    } else {
        format!("{col_name_or_ty}")
    }
}

fn to_columns_to_size_aware_tup<T>(_per_col: &Vec<T>) -> String {
    match _per_col.len() {
        1 => "columns[0].clone()".to_string(),
        n => {
            (0..n)
                .map(|m| format!("columns[{m}].clone()"))
                .join(", ")
                .pipe(|s| format!("({s})"))
        }
    }
}

fn to_collected_type_constructors(size_aware_collected_ty_per_col: &Vec<String>) -> String {
    size_aware_collected_ty_per_col
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("{ty}::new(d{i}).map_err(|_| rusqlite::Error::InvalidQuery)?"))
        .join(", ")
        .pipe(|s| {
            if size_aware_collected_ty_per_col.len() <= 1 {
                s
            } else {
                format!("({s})")
            }
        })
}

fn to_size_aware_collected_generic_ty_per_col<T>(
    _per_col: &Vec<T>, 
) -> Vec<String> {
    match _per_col.len() {
        0 => {
            vec![]
        },
        1 => {
            vec![format!("T::To")]
        },
        _ => {
            _per_col
            .iter()
            .enumerate()
            .map(|(i, _)| format!("T{i}::To"))
            .collect::<Vec<_>>()
        }
    }
}

fn to_d_tup<T>(_per_col: &Vec<T>) -> Vec<String> {
    match _per_col.len() {
        1 => {
            vec!["d0".to_string()]
        },

        len => {
            (0..len)
                .map(|n| format!("d{n}"))
                .collect::<Vec<String>>()
        }
    }
}

fn create_collected_flat_and_tup_category_for_table(
    table_name: &str, 
    col_ty_per_col: &Vec<String>, 
    col_repr_ty_per_col: &Vec<String>,
    col_sql_ty_per_col: &Vec<String>,
) -> String {
    let mut mut_collected: String = "".to_string();

    // Create flat categories for the table columns
    mut_collected += &{
        format!("\
            pub trait TrFlatCat{table_name} {{\n\
            @   fn new(rows: db_backend::ColumnResults) -> Result<Self, ()> where Self: Sized;\n\
            }}\n\n\
        ")
    };

    mut_collected += &{
        col_ty_per_col
            .iter()
            .zip(col_repr_ty_per_col.iter())
            .zip(col_sql_ty_per_col.iter())
            .map(|((col_ty, _col_repr_ty), col_sql_ty)| {
                let col_result_ty = {
                    col_sql_ty
                        .replace("PK", "Pk")
                };

                format!("\
                    impl TrFlatCat{table_name} for {col_ty} {{\n\
                    @   fn new(data: db_backend::ColumnResults) -> Result<Self, ()> {{\n\
                    @       match data {{\n\
                    @           db_backend::ColumnResults::{col_result_ty}(rows) => {{\n\
                    @               Ok({col_ty} {{ rows }})\n\
                    @           }},\n\
                    @           _ => Err(()),\n\
                    @       }}\n\
                    @   }}\n\
                    }}\n\
                ")
            })
            .join("\n")
            .replace("@", " ")
    };

    mut_collected += &{
        format!("pub trait TrCat{table_name} {{ }}\n\n")
    };

    // Now we want to implement this for up to the maximum num of columns tuple arity
    // `T, (T, T), (T, T, T), ...`
    mut_collected += &{
        (0..col_ty_per_col.len())
            .map(|arity| arity + 1)
            .map(|arity| {
                let impl_stmt = {
                    match arity {
                        1 => format!("impl<T: TrFlatCat{table_name}>"),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}: TrFlatCat{table_name}"))
                                .join(", ")
                                .pipe(|s| format!("impl<{s}>"))
                        }
                    }
                };

                let impl_for_ty = {
                    match arity {
                        1 => "T".to_string(),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}"))
                                .join(", ")
                                .pipe(|s| format!("({s})"))
                        }
                    }
                };

                format!("\
                    {impl_stmt} TrCat{table_name} for {impl_for_ty} {{ }}\
                ")
            })
            .join("\n")
            .pipe(|s| format!("{s}\n\n"))
    };

    mut_collected
}

fn create_planned_flat_and_tup_category_for_joined_table(
    joined_table_name: &str, 
    sql_table_name_per_join: &Vec<String>,
    col_ty_part_per_col: &Vec<String>, 
    col_ty_per_col: &Vec<String>, 
    _col_repr_ty_per_col: &Vec<String>,
    col_sql_ty_per_col: &Vec<String>,
    col_full_sql_name_per_col: &Vec<String>,
) -> String {
    let mut mut_planned = String::new();

    mut_planned += &{
        create_planned_flat_cat_and_useful_types_for_column_use(
            joined_table_name, 
            col_ty_part_per_col, 
            col_ty_per_col, 
            col_sql_ty_per_col,
            col_full_sql_name_per_col,
        )
    };

    // Now we want to implement this for up to the maximum num of columns tuple arity
    // `T, (T, T), (T, T, T), ...`
    // and implement the connection to the backend
    mut_planned += &{
        create_planned_category_for_joined_table_with_backend_db_impl(
            joined_table_name,
            &sql_table_name_per_join,
            col_ty_per_col, 
            col_sql_ty_per_col)
    };

    mut_planned
}

fn create_planned_flat_and_tup_category_for_table(
    table_name: &str, 
    sql_table_name: &str, 
    col_ty_part_per_col: &Vec<String>, 
    col_ty_per_col: &Vec<String>, 
    _col_repr_ty_per_col: &Vec<String>,
    col_sql_ty_per_col: &Vec<String>,
) -> String {
    let mut mut_planned = String::new();

    mut_planned += &{
        create_planned_flat_cat_and_useful_types_for_column_use(
            table_name, 
            col_ty_part_per_col, 
            col_ty_per_col, 
            col_sql_ty_per_col,
        &{
            col_ty_part_per_col
                .iter()
                .map(|col_ty_part| {
                    let snake_case_col_ty_part = to_snake_case(col_ty_part);
                    format!("{sql_table_name}.{snake_case_col_ty_part}")
                })
                .collect::<Vec<_>>()
            },
        )
    };

    // Now we want to implement this for up to the maximum num of columns tuple arity
    // `T, (T, T), (T, T, T), ...`
    mut_planned += &{
        create_planned_category_for_table_with_backend_db_impl(
            table_name,
            sql_table_name,
            col_ty_per_col, 
            col_sql_ty_per_col)
    };

    mut_planned
}

fn create_planned_flat_cat_and_useful_types_for_column_use(
    table_name: &str, 
    col_ty_part_per_col: &Vec<String>, 
    col_ty_per_col: &Vec<String>, 
    col_sql_ty_per_col: &Vec<String>,
    col_full_sql_name_per_col: &Vec<String>,
) -> String {

    let mut mut_planned = String::new();

    // for collect, we need to use the flat trait from the collected category
    mut_planned += &{
        format!("\
            use crate::collected::TrFlatCat{table_name} as TrFlatCatCollected{table_name};\n\
        \n")
    };

    // Let's create the flat types category for this table
    mut_planned += &{
        format!("\
            pub trait TrFlatCat{table_name} {{\n\
            @   fn get_column_name() -> String;\n\
            @   fn get_disambiguated_column_name() -> String;\n\
            @   fn get_column_type() -> db_backend::ColumnType;\n\
            }}\n\n\
        ")
    };

    mut_planned += &{
        col_ty_part_per_col
            .iter()
            .zip(col_ty_per_col.iter())
            .zip(col_sql_ty_per_col.iter())
            .zip(col_full_sql_name_per_col.iter())
            .map(|(((col_ty_part, col_ty), col_sql_ty), col_full_sql_name)| {
                let snake_case_col_name = to_snake_case(col_ty_part);

                let col_result_ty = {
                    col_sql_ty
                        .replace("PK", "Pk")
                };

                format!("\
                    impl TrFlatCat{table_name} for {col_ty} {{\n\
                    @   fn get_column_name() -> String {{ \"{snake_case_col_name}\".to_string() }}\n\
                    @   fn get_disambiguated_column_name() -> String {{ \"{col_full_sql_name}\".to_string() }}\n\
                    @   fn get_column_type() -> db_backend::ColumnType {{ db_backend::ColumnType::{col_result_ty} }}\n\
                    }}\n\
                ")
            })
            .join("\n")
            .replace("@", " ")
    };

    // Create a mapper from flat planned cat to flat collected cat
    mut_planned += &{
        format!("\
            pub trait TrFlatCatMap{table_name} {{\n\
            @   type To: collected::TrFlatCat{table_name};\n\
            }}\n\
        \n")
            .replace("@", " ")
    };

    // Implement the mapper for all types in the flat planned cat
    mut_planned += &{
        col_ty_per_col
            .iter()
            .map(|col_ty| {
                format!("\
                    impl TrFlatCatMap{table_name} for {col_ty} {{\n\
                    @   type To = collected::{col_ty};\n\
                    }}\n\
                ")
            })
            .join("\n")
            .pipe(|s| format!("{s}\n"))
            .replace("@", " ")
    };

    // The user may want to speak of all the columns of this category
    mut_planned += &{
        col_ty_per_col
            .iter()
            .join(", ")
            .pipe(|s| match col_ty_per_col.len() {
                1 => s,
                _ => format!("({s})")
            })
            .pipe(|s| {
                format!("\
                    pub type AllFor{table_name} = {s};
                \n")
            })
    };

    // We want to include a row struct for collected data to leverage struct unpacking
    mut_planned += &{
        col_ty_per_col
            .iter()
            .map(|col_name| {
                format!(
                    "@  pub {}: String,",
                    to_snake_case(col_name),
                )
            })
            .join("\n")
            .pipe(|s| {
                format!("\
                    pub struct RowFor{table_name} {{\n\
                    {s}\n\
                    }}\n\
                \n")
            })
            .replace("@", " ")
    };

    mut_planned
}

fn create_planned_category_for_table_with_backend_db_impl(
    table_name: &str,
    sql_table_name: &str,
    col_ty_per_col: &Vec<String>, 
    col_sql_ty_per_col: &Vec<String>
) -> String {

    let mut mut_planned = String::new();

    // Create the table category
    mut_planned += &{
        format!("\
            pub trait TrCat{table_name} {{\n\
            @   type Collected: collected::TrCat{table_name};\n\
            \n\
            @   fn get_column_names() -> Vec<String>;\n\
            @   fn get_disambiguated_column_names() -> Vec<String>;\n\
            @   fn get_column_types() -> Vec<db_backend::ColumnType>;\n\
            @   fn collect(actions: Vec<db_backend::ProgramActions>) -> rusqlite::Result<Self::Collected>;\n\
            @   fn update(where_s: &str, row: RowFor{table_name}) -> rusqlite::Result<()>;\n\
            @   fn delete(where_s: &str) -> rusqlite::Result<()>;\n\
            @   fn insert(row: RowFor{table_name}) -> rusqlite::Result<()>;\n\
            }}\n\
        \n")
            .replace("@", " ")
    };

    mut_planned += &{
        (0..col_ty_per_col.len())
            .map(|arity| arity + 1)
            .map(|arity| {
                let impl_stmt = {
                    match arity {
                        1 => format!("impl<T: TrFlatCat{table_name} + TrFlatCatMap{table_name}>"),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}: TrFlatCat{table_name} + TrFlatCatMap{table_name}"))
                                .join(", ")
                                .pipe(|s| format!("impl<{s}>"))
                        }
                    }
                };

                let impl_for_ty = {
                    match arity {
                        1 => "T".to_string(),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}"))
                                .join(", ")
                                .pipe(|s| format!("({s})"))
                        }
                    }
                };

                let collected_ty = {
                    match arity {
                        1 => "T::To".to_string(),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}::To"))
                                .join(", ")
                                .pipe(|s| format!("({s})"))
                        }
                    }
                };

                // filtering all columns by the given arity
                let col_name_per_sel_col = {
                    col_ty_per_col
                        .iter()
                        .enumerate()
                        .filter(|(i, _col_name)| *i < arity)
                        .map(|(_i, col_name)| col_name)
                        .collect::<Vec<_>>()
                };

                let d_tup_per_sel_col = {
                    to_d_tup(&col_name_per_sel_col)
                };

                let d_tup_per_sel_col_s = match d_tup_per_sel_col.len() {
                    1 => d_tup_per_sel_col[0].clone(),
                    _ => d_tup_per_sel_col.join(", ").pipe(|s| format!("({s})"))
                };

                // we want to disambeguate by including the sql table name, but also
                // we do not have access to the direct columns, only T{i}::get_column_name()
                let disambiguated_col_name_call_per_sel_col = {
                    col_name_per_sel_col
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            match col_name_per_sel_col.len() {
                                1 => format!("T::get_disambiguated_column_name()"),
                                _ => format!("T{i}::get_disambiguated_column_name()"),
                            }
                        })
                        .collect::<Vec<_>>()
                };

                let disambiguated_col_name_per_sel_col_s = {
                    disambiguated_col_name_call_per_sel_col
                        .iter()
                        .join(", ")
                };

                let col_name_call_per_sel_col = {
                    col_name_per_sel_col
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            match col_name_per_sel_col.len() {
                                1 => format!("T::get_column_name()"),
                                _ => format!("T{i}::get_column_name()"),
                            }
                        })
                        .collect::<Vec<_>>()
                };

                let col_name_per_sel_col_s = {
                    col_name_call_per_sel_col
                        .iter()
                        .join(", ")
                };

                let col_sql_ty_per_sel_col = {
                    col_sql_ty_per_col
                        .iter()
                        .enumerate()
                        .filter(|(i, _col_ty)| *i < arity)
                        .map(|(_i, col_ty)| format!("{col_ty}"))
                        .collect::<Vec<_>>()
                };

                // Turn PK -> db_backend::ColumnType::Pk, etc
                // TODO: We no longer have this information. The solver only does.
                // We need to get it from the type
                let col_result_ty_per_sel_col_s = {
                    col_sql_ty_per_sel_col
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            match col_name_per_sel_col.len() {
                                1 => format!("T::get_column_type()"),
                                _ => format!("T{i}::get_column_type()"),
                            }
                        })
                        .join(", ")
                };

                // (columns[0].clone(), columns[1].clone(), ...)
                let columns_to_size_aware_tup = {
                    to_columns_to_size_aware_tup(&d_tup_per_sel_col)
                };

                let size_aware_collected_ty_per_sel_col = {
                    to_size_aware_collected_generic_ty_per_col(&col_name_per_sel_col)
                };

                let collected_type_constructors = {
                    to_collected_type_constructors(
                        &size_aware_collected_ty_per_sel_col)
                };

                format!("\
                        {impl_stmt} TrCat{table_name} for {impl_for_ty} {{\n\
                        @   type Collected = {collected_ty};\n\
                        @   \n\
                        @   fn get_column_names() -> Vec<String> {{\n\
                        @       vec![{col_name_per_sel_col_s}]\n\
                        @   }}\n\
                        @   \n\
                        @   fn get_disambiguated_column_names() -> Vec<String> {{\n\
                        @       vec![{disambiguated_col_name_per_sel_col_s}]\n\
                        @   }}\n\
                        @   \n\
                        @   fn get_column_types() -> Vec<db_backend::ColumnType> {{\n\
                        @       vec![{col_result_ty_per_sel_col_s}]\n\
                        @   }}\n\
                        @   \n\
                        @   fn collect(actions: Vec<db_backend::ProgramActions>) -> rusqlite::Result<Self::Collected> {{\n\
                        @       let {d_tup_per_sel_col_s} = {{\n\
                        @           db_backend::get_db()\n\
                        @               .fetch_columns(\n\
                        @                   \"{sql_table_name}\",\n\
                        @                   &vec![{disambiguated_col_name_per_sel_col_s}],\n\
                        @                   &vec![{col_result_ty_per_sel_col_s}],\n\
                        @                   &actions,\n\
                        @               )?\n\
                        @               .pipe(|columns| {columns_to_size_aware_tup})\n\
                        @       }};\n\
                        @       \n\
                        @       Ok({collected_type_constructors})\n\
                        @   }}\n\
                        @   \n\
                        @   fn update(where_s: &str, row: RowFor{table_name}) -> rusqlite::Result<()> {{\n\
                        @       Err(rusqlite::Error::InvalidQuery)\n\
                        @   }}\n\
                        @   \n\
                        @   fn delete(where_s: &str) -> rusqlite::Result<()> {{\n\
                        @       Err(rusqlite::Error::InvalidQuery)\n\
                        @   }}\n\
                        @   \n\
                        @   fn insert(row: RowFor{table_name}) -> rusqlite::Result<()> {{\n\
                        @       Err(rusqlite::Error::InvalidQuery)\n\
                        @   }}\n\
                        }}\n\
                        \n")
                        .replace("@", " ")
                    })
            .join("\n")
            .pipe(|s| format!("{s}\n\n"))
    };

    mut_planned
}

fn create_planned_category_for_joined_table_with_backend_db_impl(
    joined_table_name: &str,
    sql_table_name_per_join: &Vec<String>,
    col_ty_per_col: &Vec<String>, 
    col_sql_ty_per_col: &Vec<String>
) -> String {

    let mut mut_planned = String::new();

    // Create the table category
    mut_planned += &{
        format!("\
            pub trait TrCat{joined_table_name} {{\n\
            @   type Collected: collected::TrCat{joined_table_name};\n\
            \n\
            @   fn get_column_names() -> Vec<String>;\n\
            @   fn get_disambiguated_column_names() -> Vec<String>;\n\
            @   fn get_column_types() -> Vec<db_backend::ColumnType>;\n\
            @   fn collect(actions: Vec<db_backend::ProgramActions>, keys: Vec<String>) -> rusqlite::Result<Self::Collected>;\n\
            @   fn update(where_s: &str, row: RowFor{joined_table_name}, keys: Vec<String>) -> rusqlite::Result<()>;\n\
            @   fn delete(where_s: &str, keys: Vec<String>) -> rusqlite::Result<()>;\n\
            @   fn insert(row: RowFor{joined_table_name}, keys: Vec<String>) -> rusqlite::Result<()>;\n\
            }}\n\
        \n")
            .replace("@", " ")
    };

    mut_planned += &{
        (0..col_ty_per_col.len())
            .map(|arity| arity + 1)
            .map(|arity| {
                let impl_stmt = {
                    match arity {
                        1 => format!("impl<T: TrFlatCat{joined_table_name} + TrFlatCatMap{joined_table_name}>"),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}: TrFlatCat{joined_table_name} + TrFlatCatMap{joined_table_name}"))
                                .join(", ")
                                .pipe(|s| format!("impl<{s}>"))
                        }
                    }
                };

                let impl_for_ty = {
                    match arity {
                        1 => "T".to_string(),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}"))
                                .join(", ")
                                .pipe(|s| format!("({s})"))
                        }
                    }
                };

                let collected_ty = {
                    match arity {
                        1 => "T::To".to_string(),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}::To"))
                                .join(", ")
                                .pipe(|s| format!("({s})"))
                        }
                    }
                };

                // filtering all columns by the given arity
                let col_name_per_sel_col = {
                    col_ty_per_col
                        .iter()
                        .enumerate()
                        .filter(|(i, _col_name)| *i < arity)
                        .map(|(_i, col_name)| col_name)
                        .collect::<Vec<_>>()
                };

                let d_tup_per_sel_col = {
                    to_d_tup(&col_name_per_sel_col)
                };

                let d_tup_per_sel_col_s = match d_tup_per_sel_col.len() {
                    1 => d_tup_per_sel_col[0].clone(),
                    _ => d_tup_per_sel_col.join(", ").pipe(|s| format!("({s})"))
                };

                // we want to disambeguate by including the sql table name, but also
                // we do not have access to the direct columns, only T{i}::get_column_name()
                let disambiguated_col_name_call_per_sel_col = {
                    col_name_per_sel_col
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            match col_name_per_sel_col.len() {
                                1 => format!("T::get_disambiguated_column_name()"),
                                _ => format!("T{i}::get_disambiguated_column_name()"),
                            }
                        })
                        .collect::<Vec<_>>()
                };

                let disambiguated_col_name_per_sel_col_s = {
                    disambiguated_col_name_call_per_sel_col
                        .iter()
                        .join(", ")
                };

                let col_name_call_per_sel_col = {
                    col_name_per_sel_col
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            match col_name_per_sel_col.len() {
                                1 => format!("T::get_column_name()"),
                                _ => format!("T{i}::get_column_name()"),
                            }
                        })
                        .collect::<Vec<_>>()
                };

                let col_name_per_sel_col_s = {
                    col_name_call_per_sel_col
                        .iter()
                        .join(", ")
                };

                let col_sql_ty_per_sel_col = {
                    col_sql_ty_per_col
                        .iter()
                        .enumerate()
                        .filter(|(i, _col_ty)| *i < arity)
                        .map(|(_i, col_ty)| format!("{col_ty}"))
                        .collect::<Vec<_>>()
                };

                // Turn PK -> db_backend::ColumnType::Pk, etc
                // TODO: We no longer have this information. The solver only does.
                // We need to get it from the type
                let col_result_ty_per_sel_col_s = {
                    col_sql_ty_per_sel_col
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            match col_name_per_sel_col.len() {
                                1 => format!("T::get_column_type()"),
                                _ => format!("T{i}::get_column_type()"),
                            }
                        })
                        .join(", ")
                };

                // (columns[0].clone(), columns[1].clone(), ...)
                let columns_to_size_aware_tup = {
                    to_columns_to_size_aware_tup(&d_tup_per_sel_col)
                };

                let size_aware_collected_ty_per_sel_col = {
                    to_size_aware_collected_generic_ty_per_col(&col_name_per_sel_col)
                };

                let collected_type_constructors = {
                    to_collected_type_constructors(
                        &size_aware_collected_ty_per_sel_col)
                };

                let sql_table_name = sql_table_name_per_join.first().unwrap();
                let joining_table_name_per_join_s = {
                    sql_table_name_per_join
                        .clone()
                        .into_iter()
                        .skip(1)
                        .map(|s| format!("\"{s}\".to_string()"))
                        .join(", ")
                };

                format!("\
                        {impl_stmt} TrCat{joined_table_name} for {impl_for_ty} {{\n\
                        @   type Collected = {collected_ty};\n\
                        @   \n\
                        @   fn get_column_names() -> Vec<String> {{\n\
                        @       vec![{col_name_per_sel_col_s}]\n\
                        @   }}\n\
                        @   \n\
                        @   fn get_disambiguated_column_names() -> Vec<String> {{\n\
                        @       vec![{disambiguated_col_name_per_sel_col_s}]\n\
                        @   }}\n\
                        @   \n\
                        @   fn get_column_types() -> Vec<db_backend::ColumnType> {{\n\
                        @       vec![{col_result_ty_per_sel_col_s}]\n\
                        @   }}\n\
                        @   \n\
                        @   fn collect(actions: Vec<db_backend::ProgramActions>, keys: Vec<String>) -> rusqlite::Result<Self::Collected> {{\n\
                        @       let {d_tup_per_sel_col_s} = {{\n\
                        @           db_backend::get_db()\n\
                        @               .fetch_inner_joined_columns(\n\
                        @                   \"{sql_table_name}\",\n\
                        @                   &vec![{joining_table_name_per_join_s}],\n\
                        @                   &keys,\n\
                        @                   &vec![{disambiguated_col_name_per_sel_col_s}],\n\
                        @                   &vec![{col_result_ty_per_sel_col_s}],\n\
                        @                   &actions,\n\
                        @               )?\n\
                        @               .pipe(|columns| {columns_to_size_aware_tup})\n\
                        @       }};\n\
                        @       \n\
                        @       Ok({collected_type_constructors})\n\
                        @   }}\n\
                        @   \n\
                        @   fn update(where_s: &str, row: RowFor{joined_table_name}, keys: Vec<String>) -> rusqlite::Result<()> {{\n\
                        @       Err(rusqlite::Error::InvalidQuery)\n\
                        @   }}\n\
                        @   \n\
                        @   fn delete(where_s: &str, keys: Vec<String>) -> rusqlite::Result<()> {{\n\
                        @       Err(rusqlite::Error::InvalidQuery)\n\
                        @   }}\n\
                        @   \n\
                        @   fn insert(row: RowFor{joined_table_name}, keys: Vec<String>) -> rusqlite::Result<()> {{\n\
                        @       Err(rusqlite::Error::InvalidQuery)\n\
                        @   }}\n\
                        }}\n\
                        \n")
                        .replace("@", " ")
                    })
            .join("\n")
            .pipe(|s| format!("{s}\n\n"))
    };

    mut_planned
}
fn create_collected_column_structs_and_traits_for_table(
    table_name: &str, 
    _sql_table_name: &str, 
    col_ty_part_per_col: Vec<String>, 
    col_repr_ty_per_col: Vec<String>,
    col_sql_ty_per_col: Vec<String>
) -> String {
    let mut mut_collected: String = "".to_string();

    mut_collected += &begin_comment_section(table_name);

    // Define the column structs and impl the sql type traits for them
    mut_collected += &{
        col_ty_part_per_col
            .iter()
            .zip(col_repr_ty_per_col.iter())
            .zip(col_sql_ty_per_col.iter())
            .map(|((col_ty_part, ty_repr), sql_ty)| format!(
                "pub struct {table_name}{col_ty_part} {{ pub rows: Vec<{ty_repr}> }}\n\
                 impl Tr{sql_ty} for {table_name}{col_ty_part} {{ fn get_rows(&self) -> Vec<{ty_repr}> {{ self.rows.clone() }} }}"
            ))
            .join("\n\n")
            .pipe(|s| s + "\n\n")
    };

    // Now we define a category for all possible tables 
    // that can be formulated by these columns, excluding joins.
    // This will not be done with powerset but as a category in relation to the flat
    // category for linearization
    mut_collected += &{
        create_collected_flat_and_tup_category_for_table(
            &table_name, 
            &{
                col_ty_part_per_col
                    .iter()
                    .map(|s| format!("{s}"))
                    .map(|col_ty_part| {
                        format!("{table_name}{col_ty_part}")
                    })
                    .collect::<Vec<_>>()
            },
            &{
                col_repr_ty_per_col
                    .iter()
                    .map(|s| format!("{s}"))
                    .collect::<Vec<_>>()
            },
            &{
                col_sql_ty_per_col
                    .iter()
                    .map(|s| format!("{s}"))
                    .collect::<Vec<_>>()
            },
        )
    };

    mut_collected += &end_comment_section(table_name);

    mut_collected
}

/// # example data
/// - let col_name_per_col = `vec!["Id", "Username", "Role", "CreatedAt"];`
/// - let col_repr_ty_per_col = `vec!["usize", "String", "String", "String"];`
/// - let col_sql_ty_per_col = `vec!["PK", "Varchar", "Varchar", "Varchar"];`
fn create_planned_col_tys_per_table(
    table_name: &str, 
    sql_table_name: &str, 
    col_ty_part_per_col: Vec<String>, 
    col_repr_ty_per_col: Vec<String>,
    col_sql_ty_per_col: Vec<String>) -> String {

    let mut mut_planned: String = "".to_string();

    // Define the column structs and impl the sql type traits for them
    mut_planned += &{
        col_ty_part_per_col
            .iter()
            .zip(col_repr_ty_per_col.iter())
            .zip(col_sql_ty_per_col.iter())
            .map(|((col_ty_part, _typ_repr), typ_sql)| format!("\
                pub struct {table_name}{col_ty_part};\n\
                impl Tr{typ_sql} for {table_name}{col_ty_part} {{ }}"
            ))
            .join("\n\n")
            .pipe(|s| s + "\n\n")
    };

    // Now we create our table column categories for flat types and tup arities 
    // T, (T, T), (T, T, T), ...
    mut_planned += &{
        create_planned_flat_and_tup_category_for_table(
            table_name,
            sql_table_name,
            &{
                col_ty_part_per_col
                    .iter()
                    .map(|col_ty_part| format!("{col_ty_part}"))
                    .collect::<Vec<_>>()
            },
            &{
                col_ty_part_per_col
                    .iter()
                    .map(|col_ty_part| format!("{table_name}{col_ty_part}"))
                    .collect::<Vec<_>>()
            }, 
            &col_repr_ty_per_col,
            &col_sql_ty_per_col,
        )
    };


    // We need to create a TagSelected per tup arity. The final tup 
    // arity is special and contains two versions allowing/disallowing insert

    mut_planned += &{
        (0..col_ty_part_per_col.len())
            .map(|arity| arity + 1)
            .map(|arity| {
                let string_arity_tup_s = {
                    (0..arity)
                        .map(|_| "String")
                        .join(", ")
                        .pipe(|s| {
                            if arity == 1 {
                                s
                            } else {
                                format!("({s})")
                            }
                        })
                };

                let string_arity_tup_s_paren = {
                    match arity {
                        1 => string_arity_tup_s.clone(),
                        _ => format!("({string_arity_tup_s})")
                    }
                };

                let var_row_arity_tup_s = {
                    (0..arity)
                        .map(|i| {
                            if arity == 1 {
                                format!("row.clone()")
                            } else {
                                format!("row.{i}.clone()")
                            }
                        })
                        .join(", ")
                };

                let expanded_indexed_vec_tup_s = {
                    match arity {
                        1 => format!("v[0].clone()"),
                        _ => {
                            (0..arity)
                                .map(|i| format!("v[{i}].clone()"))
                                .join(", ")
                                .pipe(|s| format!("({s})"))
                        }
                    }
                };

                let expanded_indexed_vec_access_per_col_s = {
                    col_ty_part_per_col
                        .iter()
                        .enumerate()
                        .map(|(i, col_ty_part)| format!("{}: v[{i}].clone()", to_snake_case(&format!("{table_name}{col_ty_part}"))))
                        .join(", ")
                };

                let expanded_indexed_row_access_per_col_s = {
                    col_ty_part_per_col
                        .iter()
                        .map(|col_ty_part| format!("row.{}", to_snake_case(&format!("{table_name}{col_ty_part}"))))
                        .join(", ")
                };

                if arity == col_ty_part_per_col.len() {
                    let impl_s = {
                        format!("\
                            pub struct TagSelected{arity}{table_name}<T: TrCat{table_name}> {{\n\
                            @   _marked: PhantomData<T>,\n\
                            @   actions: Vec<db_backend::ProgramActions>\n\
                            }}\n\
                            \n\
                            impl<T: TrCat{table_name}> TagSelected{arity}{table_name}<T> {{\n\
                            @   pub fn new(actions: Vec<db_backend::ProgramActions>) -> Self {{\n\
                            @       Self {{ _marked: PhantomData, actions }}\n\
                            @   }}\n\
                            @   \n\
                            @   pub fn filter(self, prog: impl Fn(RowFor{table_name}) -> String) -> NO_INSERT_TY<T> {{\n\
                            @       NO_INSERT_TY::<T> {{\n\
                            @           _marked: self._marked,\n\
                            @           actions: vec![\n\
                            @               self.actions,\n\
                            @               vec![\n\
                            @                   db_backend::ProgramActions::Filter(\n\
                            @                       prog(T::get_disambiguated_column_names()\n\
                            @                           .pipe(|v| RowFor{table_name} {{ {expanded_indexed_vec_access_per_col_s} }})\n\
                            @                       )\n\
                            @                   )\n\
                            @               ]].concat()\n\
                            @       }}\n\
                            @   }}\n\
                            @   \n\
                            @   pub fn map(self, prog: impl Fn(RowFor{table_name}) -> RowFor{table_name}) -> NO_INSERT_TY<T> {{\n\
                            @       NO_INSERT_TY::<T> {{\n\
                            @           _marked: self._marked,\n\
                            @           actions: vec![\n\
                            @               self.actions,\n\
                            @               vec![\n\
                            @                   db_backend::ProgramActions::Map(\n\
                            @                       prog(T::get_disambiguated_column_names()\n\
                            @                           .pipe(|v| RowFor{table_name} {{ {expanded_indexed_vec_access_per_col_s} }})\n\
                            @                       )\n\
                            @                           .pipe(|row| vec![{expanded_indexed_row_access_per_col_s}])\n\
                            @                   )\n\
                            @               ]].concat()\n\
                            @       }}\n\
                            @   }}\n\
                            @   \n\
                            @   pub fn collect(self) -> rusqlite::Result<T::Collected> {{\n\
                            @       T::collect(self.actions)\n\
                            @   }}\n\
                            @   \n\
                            @   pub fn update(self, where_s: &str, row: RowFor{table_name}) -> rusqlite::Result<Self> {{\n\
                            @       T::update(where_s, row)?;\n\
                            @       Ok(self)\n\
                            @   }}\n\
                            @   pub fn delete(self, where_s: &str) -> rusqlite::Result<Self> {{\n\
                            @       T::delete(where_s)?;\n\
                            @       Ok(self)\n\
                            @   }}\n\
                        ")
                    };

                    vec![
                        impl_s
                            .clone()
                            .pipe(|s| {
                                s + &format!("\
                                    @   pub fn insert(self, row: RowFor{table_name}) -> rusqlite::Result<Self> {{\n\
                                    @       T::insert(row)?;\n\
                                    @       Ok(self)\n\
                                    @   }}\n\
                                    }}\n\
                                \n")
                            })
                        ,
                        impl_s
                            .replace("TagSelected", "TagSelectedNoInsert")
                            .pipe(|s| {
                                s + &format!("\
                                    }}\n\
                                \n")
                            })
                        ,
                    ]
                        .iter()
                        .join("\n\n")
                        .replace("@", " ")
                        .replace("NO_INSERT_TY", &format!("TagSelectedNoInsert{arity}{table_name}"))
                } else {
                    format!("\
                        pub struct TagSelected{arity}{table_name}<T: TrCat{table_name}> {{\n\
                        @   _marked: PhantomData<T>,\n\
                        @   actions: Vec<db_backend::ProgramActions>\n\
                        }}\n\
                        \n\
                        impl<T: TrCat{table_name}> TagSelected{arity}{table_name}<T> {{\n\
                        @   pub fn new(actions: Vec<db_backend::ProgramActions>) -> Self {{\n\
                        @       Self {{ _marked: PhantomData, actions }}\n\
                        @   }}\n\
                        @   \n\
                        @   pub fn filter(self, prog: impl Fn({string_arity_tup_s}) -> String) -> Self {{\n\
                        @       Self {{\n\
                        @           _marked: self._marked,\n\
                        @           actions: vec![\n\
                        @               self.actions,\n\
                        @               vec![\n\
                        @                   db_backend::ProgramActions::Filter(\n\
                        @                       prog(T::get_disambiguated_column_names()\n\
                        @                           .pipe(|v| {expanded_indexed_vec_tup_s})\n\
                        @                       )\n\
                        @                   )\n\
                        @               ]].concat()\n\
                        @       }}\n\
                        @   }}\n\
                        @   \n\
                        @   pub fn map(self, prog: impl Fn({string_arity_tup_s}) -> {string_arity_tup_s_paren}) -> Self {{\n\
                        @       Self {{\n\
                        @           _marked: self._marked,\n\
                        @           actions: vec![\n\
                        @               self.actions,\n\
                        @               vec![\n\
                        @                   db_backend::ProgramActions::Map(\n\
                        @                       prog(T::get_disambiguated_column_names()\n\
                        @                           .pipe(|v| {expanded_indexed_vec_tup_s})\n\
                        @                           )\n\
                        @                           .pipe(|row| vec![{var_row_arity_tup_s}])\n\
                        @                   )\n\
                        @               ]].concat()\n\
                        @       }}\n\
                        @   }}\n\
                        @   \n\
                        @   pub fn collect(self) -> rusqlite::Result<T::Collected> {{\n\
                        @       T::collect(self.actions)\n\
                        @   }}\n\
                        }}\n\
                    \n")
                }
            })
            .join("\n\n")
            .replace("@", " ")
    };

    // Define tags for user's choice of select
    mut_planned += &{
        (0..col_ty_part_per_col.len())
            .map(|arity| arity + 1)
            .map(|arity| {
                let impl_stmt = {
                    match arity {
                        1 => format!("impl<T: TrFlatCat{table_name} + TrFlatCatMap{table_name}>"),
                        _ => {
                            (0..arity)
                                .map(|i| format!("T{i}: TrFlatCat{table_name} + TrFlatCatMap{table_name}"))
                                .join(", ")
                                .pipe(|s| format!("impl<{s}>"))
                        }
                    }
                };

                let type_declarations_per_arity = {
                    (0..arity)
                        .map(|i| match arity {
                            1 => {
                                format!("\
                                @   type T: TrFlatCat{table_name};\
                                ")
                            },
                            _ => {
                                format!("\
                                @   type T{i}: TrFlatCat{table_name};\
                                ")
                            },
                        })
                        .join("\n")
                        .replace("@", " ")
                };

                let type_declaration_refl_per_arity = {
                    (0..arity)
                        .map(|i| match arity {
                            1 => {
                                format!("\
                                @   type T = T;\
                                ")
                            },
                            _ => {
                                format!("\
                                @   type T{i} = T{i};\
                                ")
                            },
                        })
                        .join("\n")
                        .replace("@", " ")
                };


                let self_t_arity_tup = {
                    (0..arity)
                        .map(|i| match arity {
                            1 => format!("Self::T"),
                            _ => format!("Self::T{i}"),
                        })
                        .join(", ")
                        .pipe(|s| match arity {
                            1 => s,
                            _ => format!("({s})")
                        })
                };

                let ty_enum_tup = {
                    (0..arity)
                        .map(|i| match arity {
                            1 => format!("T"),
                            _ => format!("T{i}"),
                        })
                        .join(", ")
                        .pipe(|s| match arity {
                            1 => s,
                            _ => format!("({s})"),
                        })
                };

                format!("\
                    pub trait TrSelect{arity}{table_name} {{\n\
                    {type_declarations_per_arity}\n\
                    @   type Cat: TrCat{table_name};\n\
                    @   \n\
                    @   fn select{arity}() -> TagSelected{arity}{table_name}<Self::Cat>;\n\
                    }}\n\
                    \n\
                    {impl_stmt} TrSelect{arity}{table_name} for {ty_enum_tup} {{\n\
                    {type_declaration_refl_per_arity}\n\
                    @   type Cat = {self_t_arity_tup};\n\
                    @   \n\
                    @   fn select{arity}() -> TagSelected{arity}{table_name}<Self::Cat> {{\n\
                    @       TagSelected{arity}{table_name}::new(vec![])\n\
                    @   }}\n\
                    }}
                \n")
            })
            .join("\n")
            .replace("@", " ")
    };

    // Now let's create the table plan, which has all the arity selects and an extra
    // select that means selectN where N is the final arity
    mut_planned += &{
        let impl_s = {
            (0..col_ty_part_per_col.len())
                .map(|arity| arity + 1)
                .map(|arity| {
                    format!("\
                        @   pub fn select{arity}<T: TrSelect{arity}{table_name}>(&self) -> TagSelected{arity}{table_name}<T::Cat> {{\n\
                        @       T::select{arity}()\n\
                        @   }}\
                    \n")
                        .pipe(|s| {
                            if arity == col_ty_part_per_col.len() {
                                s + &format!("\n\
                                    @   pub fn select<T: TrSelect{arity}{table_name}>(&self) -> TagSelected{arity}{table_name}<T::Cat> {{\n\
                                    @   T::select{arity}()\n\
                                    @   }}\
                                \n")
                            } else {
                                s
                            }
                        })
                })
                .join("\n")
        };

        format!("\
            pub struct PlanFor{table_name};\n\
            \n\
            impl PlanFor{table_name} {{\n\
            {impl_s}\n\
            }}\n\
        \n")
    };
    
    mut_planned
}

/// # Arguments
/// - `db_fk_pk_graph`
///     - List of Foreign-key-table to primary-key-table.
///     - Ex: `vec![ ("UserOwnsPosts", "Users"), ("UserOwnsPosts", "Posts"), ];`
/// - `db_fk_pk_graph_key_pairs`
///     - List of specific column pairs. Foreign key first.
///     - Ex: `vec![ vec![("FromUserId", "Id")], vec![("ToPostId", "Id")], ];`
/// - `table_name_per_table`
///     - table name as it should be used in the struct name
/// - `sql_table_name_per_table`
///     - table name as it appears in the schema
/// - `column_struct_names_per_table`
///     - column names (without the table_name) as they would appear in the struct per table
///     - Ex: `vec![vec!["Id", "Username", "Role", "CreatedAt"], ...]`
/// - `column_struct_repr_types_per_table`
///     - Rust primtiive type representation
/// - `column_struct_sql_types_per_table`
///     - SQL type representation
fn create_all_possible_joined_tables_and_column_categories(
    db_fk_pk_graph: Vec<(&str, &str)>, 
    db_fk_pk_graph_key_pairs_per_elem_join: Vec<Vec<(&str, &str)>>, 
    table_name_per_table: Vec<&str>, 
    sql_table_name_per_table: Vec<&str>, 
    col_ty_part_per_col_per_table: Vec<Vec<&str>>, 
    col_repr_ty_per_col_per_table: Vec<Vec<&str>>, 
    col_sql_ty_per_col_per_table: Vec<Vec<&str>>) -> (String, String) {

    let mut mut_collected: String = "".to_string();
    let mut mut_planned: String = "".to_string();

    // First thing to figure out is every possible joint table tuple.
    // For this, we need to separate them by common source, then get the powersets of the [A, B, AB, ...] 
    // to form [(Common, A), (Common, B), (Common, A, B), ...] and finally concatenate across all common sources.

    let table_name_to_sql_table_name = {
        table_name_per_table
            .iter()
            .zip(sql_table_name_per_table.iter())
            .map(|(table_name, sql_table_name)| {
                (table_name.to_string(), sql_table_name.to_string())
            })
            .collect::<HashMap<String, String>>()
    };

    // [[[From1, A], [From1, B], [From1, A, B]], [[From2, C], ...], ...]
    let table_name_per_join_per_subset_per_from = {
        db_fk_pk_graph
            .clone()
            .into_iter()
            .into_group_map()
            .into_iter()
            .map(|(common_from, tos)| {
                tos
                    .into_iter()
                    .powerset()
                    .filter(|subsets| subsets.len() != 0)
                    .map(move |set| vec![vec![common_from], set.clone()].concat())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };

    // [[From1, A], [From1, B], [From1, A, B], [From2, C], ...]
    let table_name_per_join_part_per_any_subset = {
        table_name_per_join_per_subset_per_from
            .clone()
            .into_iter()
            .concat()
    };

    let sql_table_name_per_join_per_any_subset = {
        table_name_per_join_part_per_any_subset
            .iter()
            .map(|table_name_per_join| {
                table_name_per_join
                    .iter()
                    .map(|table_name| {
                        table_name_to_sql_table_name[&table_name.to_string()].clone()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };

    // Then for each tuple type, create a category for the powerset of the combined columns. Ensure that the FK table
    // comes before the PK one. 

    let joinable_table_names = {
        db_fk_pk_graph
            .iter()
            .flat_map(|(from ,to)| vec![*from, *to])
            .unique()
            .collect::<Vec<&str>>()
    };

    let joinable_table_name_to_col_props_per_col = {
        table_name_per_table
            .iter()
            .zip(sql_table_name_per_table.iter())
            .zip(col_ty_part_per_col_per_table.iter())
            .zip(col_repr_ty_per_col_per_table.iter())
            .zip(col_sql_ty_per_col_per_table.iter())
            .filter(|((((table_name, 
                         sql_table_name), 
                        _col_ty_part_per_col), 
                        _col_repr_ty_per_col), 
                        _col_sql_ty_per_col)| {
                joinable_table_names.contains(&table_name)
            })
            .flat_map(|((((table_name,
                          sql_table_name), 
                          col_ty_part_per_col), 
                          col_repr_ty_per_col), 
                          col_sql_ty_per_col)| {
                col_ty_part_per_col
                    .iter()
                    .zip(col_repr_ty_per_col.iter())
                    .zip(col_sql_ty_per_col.iter())
                    .map(move |((col_ty_part, col_repr_ty), col_sql_ty)| {
                        (table_name.to_string(), (
                            col_ty_part.to_string(),
                            format!("{table_name}{col_ty_part}"),
                            col_repr_ty.to_string(),
                            col_sql_ty.to_string(),
                            format!("{sql_table_name}.{}", to_snake_case(col_ty_part)),
                        ))
                    })
            })
            .into_group_map()
    };

    // Every n-tuple of table names maps to the collection of all columns in all of its joined tables

    let table_name_per_join_part_to_joined_table_name = {
        table_name_per_join_part_per_any_subset
            .iter()
            .map(|table_name_per_join_part| {
                (
                    table_name_per_join_part.clone(),
                    table_name_per_join_part
                        .join("Joins"),
                )
            })
            .collect::<HashMap<_, _>>()
    };

    let joined_table_name_per_any_subset = {
        table_name_per_join_part_per_any_subset
            .iter()
            .map(|table_name_per_join| {
                table_name_per_join_part_to_joined_table_name[table_name_per_join].clone()
            })
            .collect::<Vec<_>>()
    };

    let joined_table_name_to_col_props_per_col = {
        table_name_per_join_part_per_any_subset
            .iter()
            .zip(joined_table_name_per_any_subset.iter())
            .flat_map(|(table_name_per_join, joined_table_name)| {
                table_name_per_join
                    .iter()
                    .flat_map(|table_name|{
                        joinable_table_name_to_col_props_per_col[*table_name].clone()
                    })
                    .map(|props| (joined_table_name.clone(), props))
                    .collect::<Vec<_>>()
            })
            .into_group_map()
    };

    let joined_table_name_to_col_props_fn = {
        |joined_table_name: &String| {
            let (col_ty_part_per_col, (
                    col_ty_per_col, (
                    col_repr_ty_per_col, (
                    col_sql_ty_per_col, 
                    col_full_sql_name_per_col)))
                ): (Vec<_>, (Vec<_>, (Vec<_>, (Vec<_>, Vec<_>)))) = {
                joined_table_name_to_col_props_per_col[joined_table_name]
                    .clone()
                    .into_iter()
                    .map(|(a0, a1, a2, a3, a4)| (a0, (a1, (a2, (a3, (a4))))))
                    .unzip()
            };

            (
                col_ty_part_per_col, 
                col_ty_per_col,
                col_repr_ty_per_col,
                col_sql_ty_per_col,
                col_full_sql_name_per_col,
            )
        }
    };

    // debug_print(&format!("{joined_table_name_to_col_ty_per_col:?}"));

    mut_collected += &{
        begin_comment_section(joined_table_name_per_any_subset.last().unwrap())
    };

    // For each supertable, we need to create a flat category and then a category in collected
    // For each supertable, we need to create a category for each n-arity of the flat category
    mut_collected += &{
        joined_table_name_per_any_subset
            .iter()
            .map(|joined_table_name| {
                let (col_ty_part_per_col, 
                     col_ty_per_col, 
                     col_repr_ty_per_col, 
                     col_sql_ty_per_col, 
                     col_full_sql_name_per_col,) = {
                    joined_table_name_to_col_props_fn(joined_table_name)
                };

                create_collected_flat_and_tup_category_for_table(
                    joined_table_name,
                    &col_ty_per_col,
                    &col_repr_ty_per_col,
                    &col_sql_ty_per_col,
                )
            })
            .join("\n\n")
    };

    mut_collected += &{
        end_comment_section(joined_table_name_per_any_subset.last().unwrap())
    };

    mut_planned += &{
        begin_comment_section(joined_table_name_per_any_subset.last().unwrap())
    };

    mut_planned += &{
        joined_table_name_per_any_subset
            .iter()
            .zip(sql_table_name_per_join_per_any_subset.iter())
            .map(|(joined_table_name, sql_table_name_per_join)| {
                let (col_ty_part_per_col, 
                     col_ty_per_col, 
                     col_repr_ty_per_col, 
                     col_sql_ty_per_col, 
                     col_full_sql_name_per_col,) = {
                    joined_table_name_to_col_props_fn(joined_table_name)
                };

                create_planned_flat_and_tup_category_for_joined_table(
                    joined_table_name,
                    sql_table_name_per_join,
                    &col_ty_part_per_col,
                    &col_ty_per_col,
                    &col_repr_ty_per_col,
                    &col_sql_ty_per_col,
                    &col_full_sql_name_per_col,
                )
            })
            .join("\n\n")
    };

    // We need to create a category for every possible key pair per elementary join
    // TODO: If the key chain count is exactly 1, skip this. Just hardcode it and let the user do
    // .inner_join() for free. It is obvious the choice.
    let elem_join_table_name_tup_to_key_pairs = {
        db_fk_pk_graph_key_pairs_per_elem_join
            .iter()
            .zip(db_fk_pk_graph.iter())
            .map(|(db_fk_pk_graph_key_pairs, 
                  (from_fk_table_name, to_pk_table_name))| {
                ((from_fk_table_name.to_string(), to_pk_table_name.to_string()), db_fk_pk_graph_key_pairs.clone())
            })
            .collect::<HashMap<_, _>>()
    };


    let joined_table_name_to_key_pairs_per_to_join_part = {
        table_name_per_join_part_per_any_subset
            .iter()
            .map(|table_name_per_join_part| {
                (
                    table_name_per_join_part_to_joined_table_name[table_name_per_join_part].clone(),
                    table_name_per_join_part
                        .iter()
                        .skip(1)
                        .map(|joining_table_name| {
                            elem_join_table_name_tup_to_key_pairs[&(
                                table_name_per_join_part[0].to_string(),
                                joining_table_name.to_string(),
                            )].clone()
                        })
                        .collect::<Vec<_>>()
                )
            })
            .collect::<HashMap<_, _>>()
    };

    let joined_table_name_to_has_only_one_key_pair_chain = {
        table_name_per_join_part_per_any_subset
            .iter()
            .map(|table_name_per_join_part| {
                (
                    table_name_per_join_part_to_joined_table_name[table_name_per_join_part].clone(),
                    table_name_per_join_part
                )
            })
            .map(|(joined_table_name, table_name_per_join_part)| {
                (
                    joined_table_name.clone(),
                    joined_table_name_to_key_pairs_per_to_join_part[&joined_table_name]
                        .iter()
                        .all(|key_pairs| key_pairs.len() == 1)
                )
            })
            .collect::<HashMap<_, _>>()
    };

    mut_planned += &{
        table_name_per_join_part_per_any_subset
            .iter()
            .filter(|table_name_per_join| table_name_per_join.len() == 2)
            .map(|table_name_per_join| match table_name_per_join[..] {
                [from_fk_table_name, to_pk_table_name] => {
                    let joined_table_name = {
                        table_name_per_join_part_to_joined_table_name[&vec![from_fk_table_name, to_pk_table_name]]
                            .clone()
                    };

                    if !joined_table_name_to_has_only_one_key_pair_chain[&joined_table_name] {
                        format!("\
                            pub trait TrCatJoinKeys{joined_table_name} {{\n\
                            @   type FK: TrFlatCat{from_fk_table_name};\n\
                            @   type PK: TrFlatCat{to_pk_table_name};\n\
                            }}\n\
                        \n")
                            .replace("@", " ")
                    } else {
                        // We do not need it
                        format!("// TrCatJoinKeys{joined_table_name} NOT INCLUDED")
                    }
                },
                _ => unreachable!()
            })
            .join("\n\n")
            .pipe(|s| format!("{s}\n\n"))
    };

    mut_planned += &{
        table_name_per_join_part_per_any_subset
            .iter()
            .filter(|table_name_per_join| table_name_per_join.len() == 2)
            .map(|table_name_per_join| match table_name_per_join[..] {
                [from_fk_table_name, to_pk_table_name] => {
                    let joined_table_name = {
                        table_name_per_join_part_to_joined_table_name[&vec![from_fk_table_name, to_pk_table_name]]
                            .clone()
                    };

                    if !joined_table_name_to_has_only_one_key_pair_chain[&joined_table_name] {
                        let key_pairs = {
                            elem_join_table_name_tup_to_key_pairs[&(
                                from_fk_table_name.to_string(),
                                to_pk_table_name.to_string(),
                            )].clone()
                        };

                        key_pairs
                            .iter()
                            .map(|(fk, pk)| {
                                format!("\
                                    impl TrCatJoinKeys{from_fk_table_name}{to_pk_table_name} for ({from_fk_table_name}{fk}, {to_pk_table_name}{pk}) {{\n\
                                    @   type FK = {from_fk_table_name}{fk};\n\
                                    @   type PK = {to_pk_table_name}{pk};\n\
                                    }}\n\
                                \n")
                                    .replace("@", " ")
                            })
                            .join("\n\n")
                            .pipe(|s| format!("{s}\n"))

                    } else {
                        // We do not need it
                        format!("// TrCatJoinKeys{joined_table_name} IMPL NOT INCLUDED")
                    }

                },
                _ => unreachable!()
            })
            .join("\n\n")
            .pipe(|s| format!("{s}\n\n"))
    };
                
    // Now we need to do the final setup to connect this to the user categories
    mut_planned += &{
        joined_table_name_per_any_subset
            .iter()
            .zip(table_name_per_join_part_per_any_subset.iter())
            .zip(sql_table_name_per_join_per_any_subset.iter())
            .map(|((joined_table_name, table_name_per_join_part), sql_table_name_per_join)| {
                let (col_ty_part_per_col, 
                     col_ty_per_col, 
                     col_repr_ty_per_col, 
                     col_sql_ty_per_col, 
                     col_full_sql_name_per_col,) = {
                    joined_table_name_to_col_props_fn(joined_table_name)
                };

                let mut mut_planned: String = "".to_string();

                // We need to create a TagSelected per tup arity. The final tup 
                // arity is special and contains two versions allowing/disallowing insert

                mut_planned += &{
                    (0..col_ty_part_per_col.len())
                        .map(|arity| arity + 1)
                        .map(|arity| {
                            let string_arity_tup_s = {
                                (0..arity)
                                    .map(|_| "String")
                                    .join(", ")
                                    .pipe(|s| {
                                        if arity == 1 {
                                            s
                                        } else {
                                            format!("({s})")
                                        }
                                    })
                            };

                            let string_arity_tup_s_paren = {
                                match arity {
                                    1 => string_arity_tup_s.clone(),
                                    _ => format!("({string_arity_tup_s})")
                                }
                            };

                            let var_row_arity_tup_s = {
                                (0..arity)
                                    .map(|i| {
                                        if arity == 1 {
                                            format!("row.clone()")
                                        } else {
                                            format!("row.{i}.clone()")
                                        }
                                    })
                                    .join(", ")
                            };

                            let expanded_indexed_vec_tup_s = {
                                match arity {
                                    1 => format!("v[0].clone()"),
                                    _ => {
                                        (0..arity)
                                            .map(|i| format!("v[{i}].clone()"))
                                            .join(", ")
                                            .pipe(|s| format!("({s})"))
                                    }
                                }
                            };

                            let expanded_indexed_vec_access_per_col_s = {
                                col_ty_per_col
                                    .iter()
                                    .enumerate()
                                    .map(|(i, col_ty)| format!("{}: v[{i}].clone()", to_snake_case(col_ty)))
                                    .join(", ")
                            };

                            let expanded_indexed_row_access_per_col_s = {
                                col_ty_per_col
                                    .iter()
                                    .map(|col_ty| format!("row.{}", to_snake_case(col_ty)))
                                    .join(", ")
                            };

                            if arity == col_ty_part_per_col.len() {
                                let impl_s = {
                                    format!("\
                                        pub struct TagSelected{arity}{joined_table_name}<T: TrCat{joined_table_name}> {{\n\
                                        @   _marked: PhantomData<T>,\n\
                                        @   actions: Vec<db_backend::ProgramActions>,\n\
                                        @   keys: Vec<String>,\n\
                                        }}\n\
                                        \n\
                                        impl<T: TrCat{joined_table_name}> TagSelected{arity}{joined_table_name}<T> {{\n\
                                        @   pub fn new(actions: Vec<db_backend::ProgramActions>, keys: Vec<String>) -> Self {{\n\
                                        @       Self {{ _marked: PhantomData, actions, keys }}\n\
                                        @   }}\n\
                                        @   \n\
                                        @   pub fn filter(self, prog: impl Fn(RowFor{joined_table_name}) -> String) -> NO_INSERT_TY<T> {{\n\
                                        @       NO_INSERT_TY::<T> {{\n\
                                        @           _marked: self._marked,\n\
                                        @           actions: vec![\n\
                                        @               self.actions,\n\
                                        @               vec![\n\
                                        @                   db_backend::ProgramActions::Filter(\n\
                                        @                       prog(T::get_disambiguated_column_names()\n\
                                        @                           .pipe(|v| RowFor{joined_table_name} {{ {expanded_indexed_vec_access_per_col_s} }})\n\
                                        @                       )\n\
                                        @                   )\n\
                                        @               ]].concat(),\n\
                                        @               keys: self.keys,\n\
                                        @       }}\n\
                                        @   }}\n\
                                        @   \n\
                                        @   pub fn map(self, prog: impl Fn(RowFor{joined_table_name}) -> RowFor{joined_table_name}) -> NO_INSERT_TY<T> {{\n\
                                        @       NO_INSERT_TY::<T> {{\n\
                                        @           _marked: self._marked,\n\
                                        @           actions: vec![\n\
                                        @               self.actions,\n\
                                        @               vec![\n\
                                        @                   db_backend::ProgramActions::Map(\n\
                                        @                       prog(T::get_disambiguated_column_names()\n\
                                        @                           .pipe(|v| RowFor{joined_table_name} {{ {expanded_indexed_vec_access_per_col_s} }})\n\
                                        @                       )\n\
                                        @                           .pipe(|row| vec![{expanded_indexed_row_access_per_col_s}])\n\
                                        @                   )\n\
                                        @               ]].concat(),\n\
                                        @               keys: self.keys,\n\
                                        @       }}\n\
                                        @   }}\n\
                                        @   \n\
                                        @   pub fn collect(self) -> rusqlite::Result<T::Collected> {{\n\
                                        @       T::collect(self.actions, self.keys)\n\
                                        @   }}\n\
                                        @   \n\
                                        @   pub fn update(self, where_s: &str, row: RowFor{joined_table_name}) -> rusqlite::Result<Self> {{\n\
                                        @       T::update(where_s, row, self.keys.clone())?;\n\
                                        @       Ok(self)\n\
                                        @   }}\n\
                                        @   pub fn delete(self, where_s: &str) -> rusqlite::Result<Self> {{\n\
                                        @       T::delete(where_s, self.keys.clone())?;\n\
                                        @       Ok(self)\n\
                                        @   }}\n\
                                    ")
                                };

                                vec![
                                    impl_s
                                        .clone()
                                        .pipe(|s| {
                                            s + &format!("\
                                                @   pub fn insert(self, row: RowFor{joined_table_name}) -> rusqlite::Result<Self> {{\n\
                                                @       T::insert(row, self.keys.clone())?;\n\
                                                @       Ok(self)\n\
                                                @   }}\n\
                                                }}\n\
                                            \n")
                                        })
                                    ,
                                    impl_s
                                        .replace("TagSelected", "TagSelectedNoInsert")
                                        .pipe(|s| {
                                            s + &format!("\
                                                }}\n\
                                            \n")
                                        })
                                    ,
                                ]
                                    .iter()
                                    .join("\n\n")
                                    .replace("@", " ")
                                    .replace("NO_INSERT_TY", &format!("TagSelectedNoInsert{arity}{joined_table_name}"))
                            } else {
                                format!("\
                                    pub struct TagSelected{arity}{joined_table_name}<T: TrCat{joined_table_name}> {{\n\
                                    @   _marked: PhantomData<T>,\n\
                                    @   actions: Vec<db_backend::ProgramActions>,\n\
                                    @   keys: Vec<String>,\n\
                                    }}\n\
                                    \n\
                                    impl<T: TrCat{joined_table_name}> TagSelected{arity}{joined_table_name}<T> {{\n\
                                    @   pub fn new(actions: Vec<db_backend::ProgramActions>, keys: Vec<String>) -> Self {{\n\
                                    @       Self {{ _marked: PhantomData, actions, keys}}\n\
                                    @   }}\n\
                                    @   \n\
                                    @   pub fn filter(self, prog: impl Fn({string_arity_tup_s}) -> String) -> Self {{\n\
                                    @       Self {{\n\
                                    @           _marked: self._marked,\n\
                                    @           actions: vec![\n\
                                    @               self.actions,\n\
                                    @               vec![\n\
                                    @                   db_backend::ProgramActions::Filter(\n\
                                    @                       prog(T::get_disambiguated_column_names()\n\
                                    @                           .pipe(|v| {expanded_indexed_vec_tup_s})\n\
                                    @                       )\n\
                                    @                   )\n\
                                    @               ]].concat(),\n\
                                    @           keys: self.keys,\n\
                                    @       }}\n\
                                    @   }}\n\
                                    @   \n\
                                    @   pub fn map(self, prog: impl Fn({string_arity_tup_s}) -> {string_arity_tup_s_paren}) -> Self {{\n\
                                    @       Self {{\n\
                                    @           _marked: self._marked,\n\
                                    @           actions: vec![\n\
                                    @               self.actions,\n\
                                    @               vec![\n\
                                    @                   db_backend::ProgramActions::Map(\n\
                                    @                       prog(T::get_disambiguated_column_names()\n\
                                    @                           .pipe(|v| {expanded_indexed_vec_tup_s})\n\
                                    @                           )\n\
                                    @                           .pipe(|row| vec![{var_row_arity_tup_s}])\n\
                                    @                   )\n\
                                    @               ]].concat(),\n\
                                    @           keys: self.keys,\n\
                                    @       }}\n\
                                    @   }}\n\
                                    @   \n\
                                    @   pub fn collect(self) -> rusqlite::Result<T::Collected> {{\n\
                                    @       T::collect(self.actions, self.keys)\n\
                                    @   }}\n\
                                    }}\n\
                                \n")
                            }
                        })
                        .join("\n\n")
                        .replace("@", " ")
                };

                // Define tags for user's choice of select
                mut_planned += &{
                    (0..col_ty_part_per_col.len())
                        .map(|arity| arity + 1)
                        .map(|arity| {
                            let impl_stmt = {
                                match arity {
                                    1 => format!("impl<T: TrFlatCat{joined_table_name} + TrFlatCatMap{joined_table_name}>"),
                                    _ => {
                                        (0..arity)
                                            .map(|i| format!("T{i}: TrFlatCat{joined_table_name} + TrFlatCatMap{joined_table_name}"))
                                            .join(", ")
                                            .pipe(|s| format!("impl<{s}>"))
                                    }
                                }
                            };

                            let type_declarations_per_arity = {
                                (0..arity)
                                    .map(|i| match arity {
                                        1 => {
                                            format!("\
                                            @   type T: TrFlatCat{joined_table_name};\
                                            ")
                                        },
                                        _ => {
                                            format!("\
                                            @   type T{i}: TrFlatCat{joined_table_name};\
                                            ")
                                        },
                                    })
                                    .join("\n")
                                    .replace("@", " ")
                            };

                            let type_declaration_refl_per_arity = {
                                (0..arity)
                                    .map(|i| match arity {
                                        1 => {
                                            format!("\
                                            @   type T = T;\
                                            ")
                                        },
                                        _ => {
                                            format!("\
                                            @   type T{i} = T{i};\
                                            ")
                                        },
                                    })
                                    .join("\n")
                                    .replace("@", " ")
                            };


                            let self_t_arity_tup = {
                                (0..arity)
                                    .map(|i| match arity {
                                        1 => format!("Self::T"),
                                        _ => format!("Self::T{i}"),
                                    })
                                    .join(", ")
                                    .pipe(|s| match arity {
                                        1 => s,
                                        _ => format!("({s})")
                                    })
                            };

                            let ty_enum_tup = {
                                (0..arity)
                                    .map(|i| match arity {
                                        1 => format!("T"),
                                        _ => format!("T{i}"),
                                    })
                                    .join(", ")
                                    .pipe(|s| match arity {
                                        1 => s,
                                        _ => format!("({s})"),
                                    })
                            };

                            format!("\
                                pub trait TrSelect{arity}{joined_table_name} {{\n\
                                {type_declarations_per_arity}\n\
                                @   type Cat: TrCat{joined_table_name};\n\
                                @   \n\
                                @   fn select{arity}(keys: Vec<String>) -> TagSelected{arity}{joined_table_name}<Self::Cat>;\n\
                                }}\n\
                                \n\
                                {impl_stmt} TrSelect{arity}{joined_table_name} for {ty_enum_tup} {{\n\
                                {type_declaration_refl_per_arity}\n\
                                @   type Cat = {self_t_arity_tup};\n\
                                @   \n\
                                @   fn select{arity}(keys: Vec<String>) -> TagSelected{arity}{joined_table_name}<Self::Cat> {{\n\
                                @       TagSelected{arity}{joined_table_name}::new(vec![], keys)\n\
                                @   }}\n\
                                }}
                            \n")
                        })
                        .join("\n")
                        .replace("@", " ")
                };


                // Create the TagInnerJoined which presents choice for the selects after choice of inner join
                // Note that if the TagInnerJoined is non-elementary (3-path or greater A->B->C inner joins)
                // then it take multiple arity pairs of the TrCatJoinKeys as they are based on the elementary A->B pairs
                let tr_cat_join_key_type_vars_per_arity_s = {
                    table_name_per_join_part
                        .iter()
                        .skip(1)
                        .enumerate()
                        .map(|(i, to_table_name)| {
                            format!("T{i}: TrCatJoinKeys{}{to_table_name}", table_name_per_join_part[0])
                        })
                        .join(", ")
                };

                let t_per_arity_s = {
                    table_name_per_join_part
                        .iter()
                        .skip(1)
                        .enumerate()
                        .map(|(i, to_table_name)| {
                            format!("T{i}")
                        })
                        .join(", ")
                };

                let marked_vars_per_arity_s = {
                    table_name_per_join_part
                        .iter()
                        .skip(1)
                        .enumerate()
                        .map(|(i, to_table_name)| {
                            format!("_marked{i}: PhantomData<T{i}>")
                        })
                        .join(", ")
                };

                let marked_vars_construct_per_arity_s = {
                    table_name_per_join_part
                        .iter()
                        .skip(1)
                        .enumerate()
                        .map(|(i, to_table_name)| {
                            format!("_marked{i}: PhantomData")
                        })
                        .join(", ")
                };

                let key_pairs_per_to_join_part = {
                    joined_table_name_to_key_pairs_per_to_join_part[joined_table_name].clone()
                };

                let key_pairs_to_column_names_s = {
                    if !joined_table_name_to_has_only_one_key_pair_chain[joined_table_name] {
                        key_pairs_per_to_join_part
                            .iter()
                            .flat_map(|key_pairs| {
                                key_pairs
                                    .iter()
                                    .flat_map(|(fk, pk)| vec![fk.to_string(), pk.to_string()])
                            })
                            .enumerate()
                            .map(|(i, _)| match i % 2 == 0 {
                                true => format!("T{}::FK::get_disambiguated_column_name()", i / 2),
                                false => format!("T{}::PK::get_disambiguated_column_name()", i / 2),
                            })
                            .join(", ")
                    } else {
                        key_pairs_per_to_join_part
                            .iter()
                            .map(|key_pairs| {
                                let (fk, pk) = key_pairs[0];
                                // debug_println(&format!("DDD {:?}", key_pairs));
                                // debug_println(&format!("BBB ({fk}, {pk})"));
                                // debug_println(&format!("AAA {:?}", table_name_per_join_part));
                                (fk.to_string(), pk.to_string())
                            })
                            .enumerate()
                            .map(|(i, (key_fk, key_pk))| {

                                let joining_table_name = table_name_per_join_part[i+1];
                                // debug_println(&format!("AAA {i} ({key_fk}, {key_pk}) {joining_table_name} {}", table_name_per_join_part[0]));
                                format!("<{0}{key_fk} as TrFlatCat{0}>::get_disambiguated_column_name(), <{joining_table_name}{key_pk} as TrFlatCat{joining_table_name}>::get_disambiguated_column_name()", table_name_per_join_part[0])
                            })
                            .join(", ")
                    }
                };

                mut_planned += &{

                    let impl_s = {
                        (0..col_ty_part_per_col.len())
                            .map(|arity| arity + 1)
                            .map(|arity| {
                                if !joined_table_name_to_has_only_one_key_pair_chain[joined_table_name] {
                                    format!("\
                                        @   pub fn select{arity}<U: TrSelect{arity}{joined_table_name}>(&self) -> TagSelected{arity}{joined_table_name}<U::Cat> {{\n\
                                        @       U::select{arity}(vec![{key_pairs_to_column_names_s}])\n\
                                        @   }}\
                                    \n")
                                        .pipe(|s| {
                                            if arity == col_ty_part_per_col.len() {
                                                s + &format!("\n\
                                                    @   pub fn select<U: TrSelect{arity}{joined_table_name}>(&self) -> TagSelected{arity}{joined_table_name}<U::Cat> {{\n\
                                                    @       U::select{arity}(vec![{key_pairs_to_column_names_s}])\n\
                                                    @   }}\
                                                \n")
                                                    .pipe(|s| {
                                                        s + &format!("\
                                                            @   fn new() -> Self {{ Self {{ {marked_vars_construct_per_arity_s} }} }}\n\
                                                        \n")
                                                    })
                                            } else {
                                                s
                                            }
                                        })
                                } else {
                                    // We can harcode the strings, we know what they must be
                                    format!("\
                                        @   pub fn select{arity}<U: TrSelect{arity}{joined_table_name}>(&self) -> TagSelected{arity}{joined_table_name}<U::Cat> {{\n\
                                        @       U::select{arity}(vec![{key_pairs_to_column_names_s}])\n\
                                        @   }}\
                                    \n")
                                        .pipe(|s| {
                                            if arity == col_ty_part_per_col.len() {
                                                (s + &format!("\n\
                                                    @   pub fn select<U: TrSelect{arity}{joined_table_name}>(&self) -> TagSelected{arity}{joined_table_name}<U::Cat> {{\n\
                                                    @       U::select{arity}(vec![{key_pairs_to_column_names_s}])\n\
                                                    @   }}\
                                                \n"))
                                                    .pipe(|s| {
                                                        s + &format!("\
                                                            @   fn new() -> Self {{ Self {{ }} }}\n\
                                                        \n")
                                                    })
                                            } else {
                                                s
                                            }
                                        })
                                }
                            })
                            .join("\n")
                    };

                    if !joined_table_name_to_has_only_one_key_pair_chain[joined_table_name] {
                        // Category for pairs necessary, and also n-arity sensitive by inner_join path length
                        format!("\
                            pub struct TagInnerJoined{joined_table_name}<{tr_cat_join_key_type_vars_per_arity_s}> {{ {marked_vars_per_arity_s} }}\n\
                            \n\
                            impl<{tr_cat_join_key_type_vars_per_arity_s}> TagInnerJoined{joined_table_name}<{t_per_arity_s}> {{\n\
                            {impl_s}\n\
                            }}\n\
                        \n")
                    } else {
                        // No category for pairs necessary
                        format!("\
                            pub struct TagInnerJoined{joined_table_name} {{ }}\n\
                            \n\
                            impl TagInnerJoined{joined_table_name} {{\n\
                            {impl_s}\n\
                            }}\n\
                        \n")
                    }
                };

                // Now let's create the table plan, which has all the arity selects and an extra
                // select that means selectN where N is the final arity
                mut_planned += &{
                    if !joined_table_name_to_has_only_one_key_pair_chain[joined_table_name] {
                        format!("\
                            pub struct PlanFor{joined_table_name};\n\
                            \n\
                            impl PlanFor{joined_table_name} {{\n\
                            @   pub fn inner_join<{tr_cat_join_key_type_vars_per_arity_s}>(&self) -> TagInnerJoined{joined_table_name}<{t_per_arity_s}> {{\n\
                            @       TagInnerJoined{joined_table_name}::<{t_per_arity_s}>::new()
                            @   }}\n\
                            }}\n\
                        \n")
                    } else {
                        format!("\
                            pub struct PlanFor{joined_table_name};\n\
                            \n\
                            impl PlanFor{joined_table_name} {{\n\
                            @   pub fn inner_join(&self) -> TagInnerJoined{joined_table_name} {{\n\
                            @       TagInnerJoined{joined_table_name}::new()\n\
                            @   }}\n\
                            }}\n\
                        \n")
                    }
                };

                mut_planned
            })
            .join("\n\n")
    };


    mut_planned += &{
        end_comment_section(joined_table_name_per_any_subset.last().unwrap())
    };

    
    // Let's try to impl our category for the powerset of all combined table columns
    // In practice this might get very expensive real fast for rustc. This might need to be limited later.
    // let defns_per_subset = {
    //     n_tup_table_names_per_subset
    //         .iter()
    //         .zip(joined_tables_name_per_subset.iter())
    //         .map(|(n_tup_table_names, joined_table_name) | {
    //             let combined_column_tys = {
    //                 n_tup_table_names_to_column_struct_names_per_table_name[n_tup_table_names]
    //                     .clone()
    //                     .into_iter()
    //                     .concat()
    //             };

    //             create_collected_flat_and_tup_category_per_col(
    //                 joined_table_name, 
    //                 &combined_column_tys,
    //                 false,
    //                 OPT_MAX_JOINT_SUBSET_LEN,
    //             )
    //         })
    //         .collect::<Vec<_>>()
    // };

    // mut_collected += &{
    //     defns_per_subset
    //         .into_iter()
    //         .join("\n\n")
    // };

    // Now let's create the planned categories and collect implementations
    // mut_planned += &{
    //     create_planned_category_for_powerset_of_joint_columns       

    // };

    (mut_collected, mut_planned)
}

fn to_vec_string(vect: &Vec<&str>) -> Vec<String> {
    vect
        .clone()
        .into_iter()
        .map(|s| format!("{s}"))
        .collect::<Vec<_>>()
}

/// We are avoiding combinatorial explosion for selection by instead mapping between the category
/// of flat types and n-tuple types. This includes an implementation for collect, insert, and update
/// following filter and map program actions (strings the user presents). In addition, it also implements
/// inner_join to form the powerset of supertables but with linear selection of combined columns.
/// 
/// @return This gives two outputs, the left is to be put in a collected module, and the latter in planned.
fn create_db_tys() -> (String, String) {
    let mut mut_collected: String = "".to_string();
    let mut mut_planned: String = "".to_string();

    // Starting with Collected is easiest. We just have to define a struct per column with underlying
    // data representation, then we implement the category trait for all possible combinations of the columns (powerset)
    let table_name_per_table = vec![
        "Users",
        "Posts",
        "UserOwnsPosts",
    ];

    let sql_table_name_per_table = vec![
        "Users",
        "Posts",
        "User_Owns_Posts",
    ];

    // TODO: no insert/update/delete for fixed. Only insert for immutable. 
    // update/insert for mutable and update/insert/delete for ephemeral
    let _accessibility_per_table = vec![
        "fixed",
        "immutable", // default
        "mutable",
        "ephemeral",
    ];

    // TODO: Handle Extra markers on columns. Handle arbitrary feature (ex: Save), and Validation
    // markers on columns.

    // TODO: Handle custom types. They should map to primitive types, and have their own validation
    // logic. A separate file should be created as an example file for the user to copy to implement

    // TODO: Add logic to read csv files and insert according to them to a new db. Respect some 
    // markers such as Increment, Unique, Nullable, Check, custom typedef validations.
    // Note that Accessibility should be ignored. Create a admin_insert for this purpose which is
    // always available. This might be made to default to error unless specific features are 
    // enabled like INSERT_ADMIN. This will mark the distinction between db creation and program
    // execution.

    let col_ty_part_per_col_per_table = vec![
        vec!["Id", "Username", "Role", "CreatedAt"],
        vec!["Id", "Title", "Body", "Status", "CreatedAt"],
        vec!["Id", "FromUserId", "ToPostId"]
    ];

    let col_repr_ty_per_col_per_table = vec![
        vec!["usize", "String", "String", "String"],
        vec!["usize", "String", "String", "String", "String"],
        vec!["usize", "usize", "usize"]
    ];

    let col_sql_ty_per_col_per_table = vec![
        vec!["PK", "Varchar", "Varchar", "Varchar"],
        vec!["PK", "Varchar", "Varchar", "Varchar", "Varchar"],
        vec!["PK", "PK", "PK"],
    ];

    // Create traits for each primitive column type
    let sql_ty_names = {
        vec!["PK", "FK", "Int", "Varchar"]
    };

    let repr_per_sql_ty_names = {
        vec!["usize", "usize", "i64", "String"]
    };

    let db_fk_pk_graph = vec![
        ("UserOwnsPosts", "Users"),
        ("UserOwnsPosts", "Posts"),
    ];

    let db_fk_pk_graph_key_pairs_per_elem_join = vec![
        vec![("FromUserId", "Id")],
        vec![("ToPostId", "Id")],
    ];

    // Create a trait for each SQL column type. All columns can be cast to a choice
    // from here
    // TODO: Include nullables with Option<T>
    mut_collected += &{
        sql_ty_names
            .iter()
            .zip(repr_per_sql_ty_names.iter())
            .map(|(ty_name, ty)| format!("\
                pub trait Tr{ty_name} {{\n\
                @   fn get_rows(&self) -> Vec<{ty}>;\n\
                }}"
            ))
            .join("\n\n")
            .pipe(|s| s + "\n\n")
            .replace("@", " ")
    };

    // We need a sum type for all possible row type representations for runtime to use
    // TODO: look if we really need this. Likely we can just use db_backend types
    mut_collected += &{
        sql_ty_names
            .iter()
            .zip(repr_per_sql_ty_names.iter())
            .map(|(ty_name, ty)| format!("\
                @   {ty_name}({ty}),\
               "
            ))
            .join("\n")
            .pipe(|s| {
                format!("\
                    enum SQLTy {{\n\
                    {s}\n\
                    }}\n\
                ")
            })
            .pipe(|s| s + "\n\n")
            .replace("@", " ")
    };

    mut_collected += &{
        format!("\
            use crate::db_backend;\n\
        \n")
    };


    mut_collected += &{
        table_name_per_table
            .iter()
            .zip(sql_table_name_per_table.iter())
            .zip(col_ty_part_per_col_per_table.iter())
            .zip(col_repr_ty_per_col_per_table.iter())
            .zip(col_sql_ty_per_col_per_table.iter())
            .map(|((((table_name, sql_table_name), col_name_per_col), 
                      col_repr_ty_per_col), col_sql_ty_per_col)| {
                create_collected_column_structs_and_traits_for_table(
                    table_name, 
                    sql_table_name,
                    to_vec_string(col_name_per_col), 
                    to_vec_string(col_repr_ty_per_col), 
                    to_vec_string(col_sql_ty_per_col),
                )
            })
            .join("\n")
    };

    mut_planned += &{
        format!("\
            use std::marker::PhantomData;\n\
            use crate::collected;\n\
            use crate::db_backend;\n\
            use tap::prelude::*;\n\
            use itertools::Itertools;\n\
        \n")
    };

    mut_planned += &{
        sql_ty_names
            .iter()
            .zip(repr_per_sql_ty_names.iter())
            .map(|(ty_name, _repr_ty)| format!("\
                pub trait Tr{ty_name} {{ }}"
            ))
            .join("\n\n")
            .pipe(|s| s + "\n\n")
    };

    mut_planned += &{
        table_name_per_table
            .iter()
            .zip(sql_table_name_per_table.iter())
            .zip(col_ty_part_per_col_per_table.iter())
            .zip(col_repr_ty_per_col_per_table.iter())
            .zip(col_sql_ty_per_col_per_table.iter())
            .map(|((((table_name, sql_table_name), col_ty_part_per_col), 
                      col_repr_ty_per_col), 
                     col_sql_ty_per_col)| {

                let mut mut_planned = "".to_string();

                mut_planned += &begin_comment_section(table_name);

                mut_planned += &{
                    create_planned_col_tys_per_table(
                        table_name, 
                        sql_table_name,
                        to_vec_string(col_ty_part_per_col), 
                        to_vec_string(col_repr_ty_per_col), 
                        to_vec_string(col_sql_ty_per_col),
                    )
                };

                mut_planned += &end_comment_section(table_name);

                mut_planned
            })
            .join("\n")
    };

    // Now we need to support all possible joins. Joins will be expressed through the choice of plans given by the user
    // like PlanForTableAJoinTableB. Then they provide a foreign-primary key pair choice from all possible
    // choices for that type. If it's a 2-join, this is a tuple (FK, PK). Otherwise, it's multiple by n:
    // <(FK1, PK1), (FK2, PK2), ...> for .inner_join::<...>()
    // Then select and afterwards works just like normal tables. The difference is it passes extra
    // information about the keys selected by the user for the backend to do the inner join.
    // Not that (A->B->C) gives the possible supertables AB, AC, ABC. This is a powerset of the path.

    {
        create_all_possible_joined_tables_and_column_categories(
            db_fk_pk_graph,
            db_fk_pk_graph_key_pairs_per_elem_join,
            table_name_per_table,
            sql_table_name_per_table,
            col_ty_part_per_col_per_table,
            col_repr_ty_per_col_per_table,
            col_sql_ty_per_col_per_table,
        )
        .pipe(|(collected, planned)| {
            mut_collected += &collected;
            mut_planned += &planned;
        })
    };

    (mut_collected, mut_planned)
}

fn main() {
    let autogen_dir = "src/autogen";
    fs::create_dir_all(autogen_dir).expect("Failed to create autogen directory");

    let out_path = format!("{autogen_dir}/db_types.rs");
    let out_file = Path::new(&out_path);
    let mut mut_file = {
        File::create(out_file)
            .expect(&format!("Failed to create {out_path}"))
    };

    let (collected, planned) = {
        create_db_tys()
            .pipe(|(collected, planned)| {
                vec![collected, planned]
                    .iter()
                    .map(|s| {
                        s
                            .lines()
                            .map(|line| format!("    {line}"))
                            .join("\n")
                    })
                    .collect::<Vec<_>>()
                    .pipe(|vect| (vect[0].clone(), vect[1].clone()))
            })
    };

    let top_level = {
        format!("\
            pub mod collected {{\n\
            {collected}\n\
            }}\n\
            \n\
            pub mod planned {{\n\
            {planned}\n\
            }}\n\
        ").replace("@", " ")
    };

    writeln!(mut_file, "{}", top_level).unwrap();

    println!("cargo:rerun-if-changed=build.rs"); // ensures rebuild if this changes
}