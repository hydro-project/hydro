use quote::quote_spanned;

use super::{
    FloType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: An [`AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html)`<`[`Path`](https://doc.rust-lang.org/nightly/std/path/struct.Path.html)`>`
/// > for a file to read as json. This will emit the parsed value one time.
///
/// `source_json` may take one generic type argument, the type of the value to be parsed, which must implement [`Deserialize`](https://docs.rs/serde/latest/serde/de/trait.Deserialize.html).
///
/// ```dfir
/// source_json("example.json") -> for_each(|json: dfir_rs::serde_json::Value| println!("{:#?}", json));
/// ```
pub const SOURCE_JSON: OperatorConstraints = OperatorConstraints {
    name: "source_json",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    is_external_input: true,
    has_singleton_output: false,
    flo_type: Some(FloType::Source),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   op_inst:
                       OperatorInstance {
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   arguments,
                   ..
               },
               _| {
        let generic_type = type_args.first().map(|ty| quote_spanned!(op_span=> : #ty));

        let ident_jsonread = wc.make_ident("jsonread");
        let write_prologue = quote_spanned! {op_span=>
            // Note that reading the entire file to memory is (probably still) faster than using a
            // reader: https://github.com/serde-rs/json/issues/160
            let mut #ident_jsonread = {
                let string = ::std::fs::read_to_string(#arguments).unwrap();
                let value #generic_type = #root::serde_json::from_str(&string).unwrap();
                #root::futures::stream::once(value)
            };
        };
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #ident_jsonread.by_ref();
        };
        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
