use quote::quote;
use syn::Attribute;
use syn::Data;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Expr;
use syn::Lit;
use syn::Variant;

use crate::utils::doc_from;
use crate::utils::field_from;
use crate::utils::field_to_tokens;
use crate::utils::fields_from;
use crate::utils::find_attr_value;

pub fn impl_api_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<DeriveInput>(input).expect("Derive input");
    let ty = match input.data {
        Data::Struct(ref data) => api_info::Type::Struct { fields: fields_from(&data.fields) },
        Data::Enum(ref data) => enum_type(data, &input.attrs),
        _ => panic!("ApiType can only be derived for structures"),
    };
    let field = field_from(Some(&input.ident), &input.attrs, ty);
    let type_name = &input.ident;
    let field_tokens = field_to_tokens(&field);
    let tokens = quote! {
        impl api_info::ApiType for #type_name {
            fn api() -> api_info::Field {
                #field_tokens
            }
        }
    };
    tokens.into()
}

fn enum_type(data: &DataEnum, attrs: &Vec<Attribute>) -> api_info::Type {
    if data.variants.iter().any(|v| !v.fields.is_empty()) {
        enum_of_types(data, attrs)
    } else {
        enum_of_consts(data)
    }
}

fn enum_of_types(data: &DataEnum, attrs: &Vec<Attribute>) -> api_info::Type {
    let content = find_attr_value("serde", "content", attrs);
    let types = data.variants.iter().map(|v| {
        let fields = fields_from(&v.fields);
        let mut variant_type = api_info::Type::Struct { fields };
        if let Some(content) = &content {
            variant_type = api_info::Type::Struct {
                fields: vec![api_info::Field {
                    name: content.clone(),
                    summary: None,
                    description: None,
                    value: variant_type,
                }],
            };
        }
        field_from(Some(&v.ident), &v.attrs, variant_type)
    });
    api_info::Type::EnumOfTypes { types: types.collect() }
}

fn enum_of_consts(data: &DataEnum) -> api_info::Type {
    let consts = data.variants.iter().map(const_from);
    api_info::Type::EnumOfConsts { consts: consts.collect() }
}

fn const_from(v: &Variant) -> api_info::Const {
    let name = v.ident.to_string();
    let (summary, description) = doc_from(&v.attrs);
    let value = match v.discriminant.as_ref().map(|(_, e)| e) {
        Some(expr) => {
            let lit = match expr {
                Expr::Lit(expr_lit) => &expr_lit.lit,
                _ => panic!("Invalid enum const."),
            };
            value_from_lit(lit)
        }
        None => api_info::ConstValue::None {},
    };
    api_info::Const { name, value, summary, description }
}

fn value_from_lit(lit: &Lit) -> api_info::ConstValue {
    match lit {
        Lit::Bool(v) => api_info::ConstValue::Bool(if v.value { "true" } else { "false" }.into()),
        Lit::Str(v) => api_info::ConstValue::String(v.value()),
        Lit::Byte(v) => api_info::ConstValue::Number(v.value().to_string()),
        Lit::Int(v) => api_info::ConstValue::Number(v.base10_digits().into()),
        Lit::Float(v) => api_info::ConstValue::Number(v.base10_digits().into()),
        _ => panic!("Invalid enum const."),
    }
}

pub fn impl_zeroize_on_drop(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<DeriveInput>(input).expect("Derive input");
    let type_name = &input.ident;
    let tokens = quote! {
        impl Drop for #type_name {
            fn drop(&mut self) {
                self.zeroize();
            }
        }
    };
    tokens.into()
}
