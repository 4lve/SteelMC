use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, LitStr, Meta};

static ALLOWED_TYPES: [&str; 12]  = ["i8","i16","i32","i64","i128","isize","u8","u16","u32","u64","u128","usize"];

#[proc_macro_derive(PacketRead, attributes(read_as))]
pub fn packet_read_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    match input.data {
        Data::Struct(s) => {
            let Fields::Named(fields) = s.fields else {
                panic!("PacketRead only supports structs with named fields");
            };

            let readers = fields.named.iter().map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let field_type = &f.ty;
                let mut read_strategy: Option<String> = None;
                let mut bound: Option<syn::LitInt> = None;

                if let Some(attr) = f.attrs.iter().find(|a| a.path().is_ident("read_as")) {
                    if let Meta::List(meta) = attr.meta.clone() {
                        meta.parse_nested_meta(|meta| {
                            if meta.path.is_ident("as") {
                                let value = meta.value()?;
                                let s: LitStr = value.parse()?;
                                read_strategy = Some(s.value());
                                Ok(())
                            } else if meta.path.is_ident("bound") {
                                let value = meta.value()?;
                                let int_lit: syn::LitInt = value.parse()?;
                                bound = Some(int_lit);
                                Ok(())
                            } else {
                                Err(meta.error(
                                    "unsupported property in `read_as` attribute. Expected `as = \"...\"` or `bound = ...`",
                                ))
                            }
                        })
                        .unwrap_or_else(|e| panic!("Failed to parse `read_as` attribute: {}", e));
                    } else {
                        panic!("`read_as` attribute requires a list format: `#[read_as(as = \"...\", bound = ...)]`");
                    }
                }

                match read_strategy.as_deref() {
                    Some("var_int") => quote! {
                        let #field_name = data.reader().get_var_int()?;
                    },
                    Some("string") => {
                        let read_call = if let Some(b) = bound {
                            quote! { data.reader().get_string_bounded(#b)? }
                        } else {
                            quote! { data.reader().get_string()? }
                        };

                        quote! {
                            let #field_name = #read_call;
                        }
                    }
                    // This case was already correct.
                    None => quote! {
                        let #field_name = <#field_type>::read_packet(data)?;
                    },
                    Some(s) => panic!("Unknown read strategy: `{}`", s),
                }
            });

            let field_names = fields.named.iter().map(|f| f.ident.as_ref().unwrap());

            let expanded = quote! {
                #[automatically_derived]
                // The trait implementation for the struct.
                impl PacketRead for #name {
                    // FIX 3: Signature now takes `&mut bytes::Bytes` to match the trait.
                    // FIX 1: The return type uses a fully qualified path, no `use` keyword.
                    fn read_packet(data: &mut bytes::Bytes) -> Result<Self, crate::utils::PacketReadError>
                    where
                        Self: Sized,
                    {
                        use crate::ser::NetworkReadExt;
                        // Execute all generated field readers to create local variables
                        #(#readers)*

                        // Construct the struct from the read fields
                        Ok(Self {
                            #(#field_names),*
                        })
                    }
                }
            };

            TokenStream::from(expanded)
        },
        Data::Enum(e) => {
            let readers = e.variants.iter().map(|v| {
                if !matches!(v.fields, Fields::Unit) {
                    panic!("PacketReader only supports enum variants without fields");
                }
                let Some((_, value)) = &v.discriminant else {
                    panic!("PacketReader only supports enum variants with explicit discriminant\n(Ej. {} = 0)", &v.ident)
                };
                let v_name = &v.ident;
                quote! {
                    #value => #name::#v_name,
                }
            });

            let error_msg = format!("Invalid {name}");

            TokenStream::from(quote! {
                #[automatically_derived]
                impl PacketRead for #name {
                    fn read_packet(data: &mut bytes::Bytes) -> Result<Self, crate::utils::PacketReadError>
                    where
                        Self: Sized,
                    {
                        Ok(match crate::codec::var_int::read(&mut data.reader())? {
                            #(#readers)*
                            _ => {
                                return Err(crate::utils::PacketReadError::MalformedValue(
                                    #error_msg.to_string(),
                                ));
                            }
                        })
                    }
                }
            })
        },
        _ => panic!("PacketRead can only be derived for structs or enums"),
    }
}

#[proc_macro_derive(PacketWrite, attributes(write_as))]
pub fn packet_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    match input.data {
        Data::Struct(s) => {
            let Fields::Named(fields) = s.fields else {
                panic!("PacketWrite only supports structs with named fields");
            };

            let writers = fields.named.iter().map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let mut write_strategy: Option<String> = None;
                let mut bound: Option<syn::LitInt> = None;

                if let Some(attr) = f.attrs.iter().find(|a| a.path().is_ident("write_as")) {
                    if let Meta::List(meta) = attr.meta.clone() {
                        meta.parse_nested_meta(|meta| {
                            if meta.path.is_ident("as") {
                                let value = meta.value()?;
                                let s: LitStr = value.parse()?;
                                write_strategy = Some(s.value());
                                Ok(())
                            } else if meta.path.is_ident("bound") {
                                let value = meta.value()?;
                                let int_lit: syn::LitInt = value.parse()?;
                                bound = Some(int_lit);
                                Ok(())
                            } else {
                                Err(meta.error(
                                    "unsupported property. Expected `as = \"...\"` or `bound = ...`",
                                ))
                            }
                        })
                        .unwrap_or_else(|e| panic!("Failed to parse `write_as` attribute: {}", e));
                    } else {
                        panic!("`write_as` attribute requires a list format");
                    }
                }

                match write_strategy.as_deref() {
                    Some("var_int") => quote! {
                        writer.write_var_int(self.#field_name)?;
                    },
                    Some("string") => {
                        let write_call = if let Some(b) = bound {
                            quote! { writer.write_string_bounded(&self.#field_name, #b)?; }
                        } else {
                            quote! { writer.write_string(&self.#field_name)?; }
                        };

                        quote! {
                            #write_call
                        }
                    }
                    None => quote! {
                        self.#field_name.write_packet(writer)?;
                    },
                    Some(s) => panic!("Unknown write strategy: `{}`", s),
                }
            });

            let expanded = quote! {
                #[automatically_derived]
                impl PacketWrite for #name {
                    fn write_packet(&self, writer: &mut impl Write) -> Result<(), crate::utils::PacketWriteError> {
                        use std::io::Write;
                        use crate::ser::NetworkWriteExt;

                        #(#writers)*

                        Ok(())
                    }
                }
            };

            TokenStream::from(expanded)
        },
        Data::Enum(_) => {
            let mut write_strategy: Option<String> = None;
            let mut bound: Option<syn::LitInt> = None;
            if let Some(attr) = input.attrs.iter().find(|a| a.path().is_ident("write_as")) {
                if let Meta::List(meta) = attr.meta.clone() {
                    meta.parse_nested_meta(|meta| {
                        if meta.path.is_ident("as") {
                            let value = meta.value()?;
                            let s: LitStr = value.parse()?;
                            write_strategy = Some(s.value());
                            Ok(())
                        } else if meta.path.is_ident("bound") {
                            let value = meta.value()?;
                            let int_lit: syn::LitInt = value.parse()?;
                            bound = Some(int_lit);
                            Ok(())
                        } else {
                            Err(meta.error(
                                "unsupported property. Expected `as = \"...\"` or `bound = ...`",
                            ))
                        }
                    })
                    .unwrap_or_else(|e| panic!("Failed to parse `write_as` attribute: {}", e));
                } else {
                    panic!("`write_as` attribute requires a list format");
                }
            } else {panic!("PacketWrite enums requires the \"write_as\" attribute")}

            let writer = match write_strategy.as_deref() {
                // Specialiced implementation
                Some("var_int") => {
                    quote! {
                        writer.write_var_int(*self as i32)?;
                    }
                },
                // Simple implementation
                Some(s) => {
                    if !ALLOWED_TYPES.contains(&s) {
                        panic!("Unknown write strategy: `{}`", s)
                    }
                    let enum_type = Ident::new(s, Span::call_site());
                    quote! {
                        (*self as #enum_type).write_packet(writer)?;
                    }
                },
                None => panic!("Expected write_as's \"as\" value"),
            };
            TokenStream::from(quote! {
                #[automatically_derived]
                impl PacketWrite for #name {
                    fn write_packet(&self, writer: &mut impl Write) -> Result<(), crate::utils::PacketWriteError> {
                        use std::io::Write;
                        use crate::ser::NetworkWriteExt;

                        #writer

                        Ok(())
                    }
                }
            })
        }
        _ => panic!("PacketWrite can only be derived for structs and enums"),
    }
}
