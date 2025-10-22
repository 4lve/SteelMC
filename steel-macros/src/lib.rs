use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Meta, parse_macro_input};

const ALLOWED_TYPES: [&str; 12] = [
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
];

const PROPERTY_ERROR: &str = "unsupported attribute. Supported properties are `as = \"...\"`, `bound = ...`, `prefix = ...`";

#[proc_macro_derive(PacketRead, attributes(read_as))]
pub fn packet_read_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    match input.data {
        Data::Struct(s) => {
            let Fields::Named(fields) = s.fields else {
                panic!("PacketRead only supports structs with named fields");
            };

            // Create read calls for every field
            let readers = fields.named.iter().map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let field_type = &f.ty;

                let mut read_strategy: Option<String> = None;
                let mut bound: Option<syn::LitInt> = None;
                let mut prefix: Option<syn::Type> = None;

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
                            } else if meta.path.is_ident("prefix") {
                                let value = meta.value()?;
                                let type_lit: syn::Type = value.parse()?;
                                prefix = Some(type_lit);
                                Ok(())
                            } else {
                                Err(meta.error(
                                    PROPERTY_ERROR,
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
                        let #field_name = crate::codec::VarInt::read(data)? as #field_type;
                    },
                    Some("string") | Some("vec") => {
                        let prefix = prefix.unwrap_or_else(|| syn::parse_quote!(crate::codec::VarInt));

                        let read_call = if let Some(b) = bound {
                            quote! { <#field_type>::read_prefixed_bound::<#prefix>(data, #b)? }
                        } else {
                            quote! { <#field_type>::read_prefixed::<#prefix>(data)? }
                        };

                        quote! {
                            use crate::packet_traits::PrefixedRead;
                            let #field_name = #read_call;
                        }
                    },
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
                    fn read_packet(data: &mut impl std::io::Read) -> Result<Self, crate::utils::PacketReadError>
                    where
                        Self: Sized,
                    {
                        use std::io::Read;

                        #(#readers)*

                        Ok(Self {
                            #(#field_names),*
                        })
                    }
                }
            };

            TokenStream::from(expanded)
        }
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

            // Support reading the enum discriminant using a specified strategy
            // Defaults to reading a varint when no attribute is provided
            let mut read_strategy: Option<String> = None;
            let mut bound: Option<syn::LitInt> = None;
            if let Some(attr) = input.attrs.iter().find(|a| a.path().is_ident("read_as")) {
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
                            Err(meta.error(PROPERTY_ERROR))
                        }
                    })
                    .unwrap_or_else(|e| panic!("Failed to parse `read_as` attribute: {}", e));
                } else {
                    panic!(
                        "`read_as` attribute requires a list format: `#[read_as(as = \"...\", bound = ...)]`"
                    );
                }
            }

            let read_discriminant = match read_strategy.as_deref() {
                // Specialized implementation: read a VarInt (i32)
                None | Some("var_int") => quote! { crate::codec::VarInt::read(data)? },
                // Simple implementation: read as a primitive numeric type
                Some(s) => {
                    if !ALLOWED_TYPES.contains(&s) {
                        panic!("Unknown read strategy: `{}`", s)
                    }
                    let enum_type = Ident::new(s, Span::call_site());
                    let _ = bound; // `bound` currently unused for primitive reads
                    quote! { <#enum_type as PacketRead>::read_packet(data)? }
                }
            };

            let error_msg = format!("Invalid {name}");

            TokenStream::from(quote! {
                #[automatically_derived]
                impl PacketRead for #name {
                    fn read_packet(data: &mut impl std::io::Read) -> Result<Self, crate::utils::PacketReadError>
                    where
                        Self: Sized,
                    {
                        use std::io::Read;
                        use crate::ser::NetworkReadExt;
                        Ok(match { #read_discriminant } {
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
        }
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
                let mut prefix: Option<syn::Type> = None;

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
                            } else if meta.path.is_ident("prefix") {
                                let value = meta.value()?;
                                let type_lit: syn::Type = value.parse()?;
                                prefix = Some(type_lit);
                                Ok(())
                            } else {
                                Err(meta.error(PROPERTY_ERROR))
                            }
                        })
                        .unwrap_or_else(|e| panic!("Failed to parse `write_as` attribute: {}", e));
                    } else {
                        panic!("`write_as` attribute requires a list format");
                    }
                }

                match write_strategy.as_deref() {
                    Some("var_int") => quote! {
                        crate::codec::VarInt(self.#field_name).write(writer)?;
                    },
                    Some("string") | Some("vec") => {
                        let prefix = prefix.unwrap_or_else(|| syn::parse_quote!(crate::codec::VarInt));

                        let write_call = if let Some(b) = bound {
                            quote! { self.#field_name.write_prefixed_bound::<#prefix>(writer, #b)?; }
                        } else {
                            quote! { self.#field_name.write_prefixed::<#prefix>(writer)?; }
                        };

                        quote! {
                            use crate::packet_traits::PrefixedWrite;
                            #write_call
                        }
                    },
                    Some("json") => quote! {
                        writer.write_string(&serde_json::to_string(&self.#field_name).map_err(|e| {
                            crate::utils::PacketWriteError::Message(format!("Failed to serialize: {e}"))
                        })?)?;
                    },
                    None => quote! {
                        self.#field_name.write_packet(writer)?;
                    },
                    Some(s) => panic!("Unknown write strategy: `{}`", s),
                }
            });

            let expanded = quote! {
                #[automatically_derived]
                impl PacketWrite for #name {
                    fn write_packet(&self, writer: &mut impl std::io::Write) -> Result<(), crate::utils::PacketWriteError> {
                        use std::io::Write;

                        #(#writers)*

                        Ok(())
                    }
                }
            };

            TokenStream::from(expanded)
        }
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
                            Err(meta.error(PROPERTY_ERROR))
                        }
                    })
                    .unwrap_or_else(|e| panic!("Failed to parse `write_as` attribute: {}", e));
                } else {
                    panic!("`write_as` attribute requires a list format");
                }
            } else {
                panic!("PacketWrite enums requires the \"write_as\" attribute")
            }

            let writer = match write_strategy.as_deref() {
                // Specialiced implementation
                Some("var_int") => {
                    quote! {
                        writer.write_var_int(*self as i32)?;
                    }
                }
                // Simple implementation
                Some(s) => {
                    if !ALLOWED_TYPES.contains(&s) {
                        panic!("Unknown write strategy: `{}`", s)
                    }
                    let enum_type = Ident::new(s, Span::call_site());
                    quote! {
                        (*self as #enum_type).write_packet(writer)?;
                    }
                }
                None => panic!("Expected write_as's \"as\" value"),
            };
            TokenStream::from(quote! {
                #[automatically_derived]
                impl PacketWrite for #name {
                    fn write_packet(&self, writer: &mut impl std::io::Write) -> Result<(), crate::utils::PacketWriteError> {
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

#[proc_macro_attribute]
pub fn packet(input: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(item.clone()).unwrap();
    let name = &ast.ident;
    let (impl_generics, ty_generics, _) = ast.generics.split_for_impl();

    let input: proc_macro2::TokenStream = input.into();
    let item: proc_macro2::TokenStream = item.into();

    let code = quote! {
        #item
        impl #impl_generics crate::packet_traits::Packet for #name #ty_generics {
            const PACKET_ID: i32 = #input;
        }
    };

    code.into()
}
