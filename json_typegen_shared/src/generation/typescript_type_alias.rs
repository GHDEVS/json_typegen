use linked_hash_map::LinkedHashMap;

use crate::options::Options;
use crate::shape::{self, Shape};
use crate::generation::typescript::{is_ts_identifier, collapse_option};

pub struct Ctxt {
    options: Options,
    indent_level: usize,
}

pub type Ident = String;
pub type Code = String;

pub fn typescript_type_alias(name: &str, shape: &Shape, options: Options) -> (Ident, Option<Code>) {
    let mut ctxt = Ctxt {
        options,
        indent_level: 1
    };

    let mut code = type_from_shape(&mut ctxt, shape);

    code = format!("export type {} = {};\n\n", name, code);
    (name.to_string(), Some(code))
}

fn type_from_shape(ctxt: &mut Ctxt, shape: &Shape) -> Code {
    use crate::shape::Shape::*;
    match *shape {
        Null | Any | Bottom => "any".into(),
        Bool => "boolean".into(),
        StringT => "string".into(),
        Integer => "number".into(),
        Floating => "number".into(),
        Tuple(ref shapes, _n) => {
            let folded = shape::fold_shapes(shapes.clone());
            if folded == Any && shapes.iter().any(|s| s != &Any) {
                generate_tuple_type(ctxt, shapes)
            } else {
                generate_vec_type(ctxt,  &folded)
            }
        }
        VecT { elem_type: ref e } => generate_vec_type(ctxt,e),
        Struct { fields: ref map } => generate_struct_from_field_shapes(ctxt, map),
        MapT { val_type: ref v } => generate_map_type(ctxt,v),
        Opaque(ref t) => t.clone(),
        Optional(ref e) => {
            let inner = type_from_shape(ctxt,e);
            if ctxt.options.use_default_for_missing_fields {
                inner
            } else {
                format!("{} | undefined", inner)
            }
        }
    }
}

fn generate_vec_type(ctxt: &mut Ctxt, shape: &Shape) -> Code {
    let inner = type_from_shape(ctxt, &shape);
    format!("Array<{}>", inner)
}

fn generate_map_type(ctxt: &mut Ctxt, shape: &Shape) -> Code {
    let inner = type_from_shape(ctxt, &shape);
    format!("{{ [key: string]: {} }}", inner)
}

fn generate_tuple_type(ctxt: &mut Ctxt, shapes: &[Shape]) -> Code {
    let mut types = Vec::new();

    for shape in shapes {
        let typ = type_from_shape(ctxt, shape);
        types.push(typ);
    }

    format!("[{}]", types.join(", "))
}

fn generate_struct_from_field_shapes(
    ctxt: &mut Ctxt,
    map: &LinkedHashMap<String, Shape>,
) -> Code {
    let fields: Vec<Code> = map
        .iter()
        .map(|(name, typ)| {
            let (was_optional, collapsed) = collapse_option(typ);

            ctxt.indent_level += 1;
            let field_type = type_from_shape(ctxt,collapsed);
            ctxt.indent_level -= 1;

            let escape_name = !is_ts_identifier(name);

            format!(
                "{}{}{}{}{}: {};",
                "    ".repeat(ctxt.indent_level),
                if escape_name { "\"" } else { "" },
                name,
                if escape_name { "\"" } else { "" },
                if was_optional { "?" } else { "" },
                field_type
            )
        })
        .collect();

    let mut code = format!("{{\n");

    if !fields.is_empty() {
        code += &fields.join("\n");
        code += "\n";
        code += &"    ".repeat(ctxt.indent_level - 1);
    }
    code += "}";

    code
}
