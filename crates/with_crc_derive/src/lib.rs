use proc_macro::TokenStream as TokenStream1;

use quote::{format_ident, quote};
use syn::{Data, DataEnum, DeriveInput, Expr, Meta, MetaList, parse_str};

#[proc_macro_derive(WithCrc, attributes(crc))]
pub fn with_crc(item: TokenStream1) -> TokenStream1 {
    let input = syn::parse_macro_input!(item as DeriveInput);
    //println!("------------- {:#?}", input);
    let struct_name = format_ident!("{}", input.ident.to_string());

    match input.data {
        Data::Struct(_) => {
            for attr in &input.attrs {
                if let Meta::List(MetaList { path, tokens, .. }) = &attr.meta {
                    let key = path.segments[0].ident.to_string();
                    let value = tokens.to_string();
                    if key != "crc" || value.is_empty() { continue; }

                    let crc = parse_str::<Expr>(&value).unwrap();
                    let token = quote! {
                        impl with_crc::WithCrc for #struct_name {
                            fn crc(&self) -> u32 { #crc }
                        }
                    };
                    return token.into();
                }
            }
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let mut vec = Vec::new();
            for variant in variants {
                let enum_name = format_ident!("{}", variant.ident.to_string());
                let no_fields = variant.fields.is_empty();
                for attr in &variant.attrs {
                    if let Meta::List(MetaList { path, tokens, .. }) = &attr.meta {
                        let key = path.segments[0].ident.to_string();
                        let value = tokens.to_string();
                        if key != "crc" || value.is_empty() { continue; }

                        let crc = parse_str::<Expr>(&value).unwrap();
                        let token = if no_fields {
                            quote! { Self::#enum_name => #crc, }
                        } else {
                            quote! { Self::#enum_name {..} => #crc, }
                        };

                        vec.push(token);
                        break;
                    }
                }
            }
            if !vec.is_empty() {
                let token = quote! {
                    impl with_crc::WithCrc for #struct_name {
                        fn crc(&self) -> u32 {
                            match self {
                                #(#vec)*
                            }
                        }
                    }
                };
                // println!("----- {:#}", token);

                return token.into();
            }
        }
        Data::Union(_) => {}
    }

    TokenStream1::default()
}

/*#[proc_macro_attribute]
pub fn crc(attr: TokenStream1, item: TokenStream1) -> TokenStream1 {
    let crc = attr.to_string();
    if crc.is_empty() { return item; }

    println!("------- raw: {}", item.to_string());
    let input = item.clone();
    let mut input = syn::parse_macro_input!(input as Item);
    let mut visitor = CrcStructVisitor::new(crc);
    visitor.visit_item_mut(&mut input);
    let impl_default = visitor.impl_default;

    println!("--------------------------------");
    let token = quote!{
        #input
        #impl_default
    };
    println!("{:#}", token.to_string());
    println!("--------------------------------");
    token.into()
}

#[derive(Debug, Clone)]
struct CrcStructVisitor {
    crc: String,
    struct_name: String,
    impl_default: TokenStream,
}

impl CrcStructVisitor {
    pub fn new(crc: String) -> Self {
        Self {
            crc,
            struct_name: String::new(),
            impl_default: quote!(),
        }
    }
}

impl VisitMut for CrcStructVisitor {

    fn visit_fields_named_mut(&mut self, i: &mut FieldsNamed) {
        // 实现 Default
        let crc = parse_str::<Expr>(&self.crc).unwrap();
        let struct_name = format_ident!("{}", self.struct_name);
        let mut field_names = vec![];
        for field in i.named.iter() {
            let name = field.ident.clone().unwrap().to_string();
            field_names.push(format_ident!("{}", name));
        }

        self.impl_default = quote! {
            impl Default for #struct_name {
                fn default() -> Self {
                    Self {
                        crc: #crc,
                        #(#field_names:Default::default()),*
                    }
                }
            }
            impl #struct_name {
                pub fn new() -> Self {
                    Self {
                        crc: #crc,
                        #(#field_names:Default::default()),*
                    }
                }
            }
        };

        // 增加 _crc 字段
        let fields: FieldsNamed = parse_str("{crc: i64}").unwrap();
        i.named.insert(0, fields.named[0].clone());

    }

    fn visit_item_struct_mut(&mut self, i: &mut ItemStruct) {
        self.struct_name = i.ident.to_string();
        visit_mut::visit_item_struct_mut(self, i);
    }
}*/

