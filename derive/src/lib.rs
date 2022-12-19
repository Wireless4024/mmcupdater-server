use proc_macro::TokenStream;
use proc_macro2::Span;

use quote::{format_ident, quote, quote_spanned};
use syn::{Data, DeriveInput, Error, Ident, parse_macro_input, Type};
use syn::spanned::Spanned;

macro_rules! parse_struct {
    ($data:ident,$ident:ident) => {
	    match $data {
			Data::Struct(s) => { s }
			_ => {
				return Error::new($ident.span().unwrap().into(), "Not a struct")
					.into_compile_error()
					.into();
			}
		}
    };
}

#[proc_macro_derive(ValueAccess)]
pub fn derive_value_access(input: TokenStream) -> TokenStream {
	let DeriveInput { ident, data, .. } = parse_macro_input!(input);
	let s = parse_struct!(data, ident);
	let mut tokens = Vec::new();
	let mut idents = Vec::new();
	for x in &s.fields {
		if let Some(ref ident) = x.ident {
			let token = match &x.ty {
				Type::Path(p) => {
					let path = &p.path;
					if let Some(src) = path.segments.span().unwrap().source_text() {
						let s = src.as_str().rsplit("::").next().unwrap_or(src.as_str());
						match s {
							"i8" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::I8(self.#ident))}
							}
							"i16" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::I16(self.#ident))}
							}
							"i32" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::I32(self.#ident))}
							}
							"i64" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::I64(self.#ident))}
							}
							"i128" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::I128(self.#ident))}
							}
							"isize" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::ISize(self.#ident))}
							}
							"u8" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::U8(self.#ident))}
							}
							"u16" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::U16(self.#ident))}
							}
							"u32" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::U32(self.#ident))}
							}
							"u64" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::U64(self.#ident))}
							}
							"u128" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::U128(self.#ident))}
							}
							"usize" => {
								quote! {base::value::RustPrimitiveValueRef::Integer(base::value::Integer::USize(self.#ident))}
							}
							"f32" => {
								quote! {base::value::RustPrimitiveValueRef::Float(base::value::Float::F32(self.#ident))}
							}
							"f64" => {
								quote! {base::value::RustPrimitiveValueRef::Float(base::value::Float::F64(self.#ident))}
							}
							"String" => {
								quote! {base::value::RustPrimitiveValueRef::String(&self.#ident)}
							}
							"Vec<u8>" => {
								quote! {base::value::RustPrimitiveValueRef::Bytes(&self.#ident)}
							}
							_ => {
								continue;
							}
						}
					} else {
						continue;
					}
				}
				_ => {
					Error::new(x.span().unwrap().into(), "Type is not supported")
						.into_compile_error()
				}
			};
			tokens.push(token);
			idents.push(ident);
		}
	}
	let output = quote! {
	        impl base::value::ValueAccess for #ident {
				fn get_value(&self, key: &str) -> Option<base::value::RustPrimitiveValueRef> {
					match key {
						#( stringify!(#idents) => Some(#tokens), )*
						_ => None
					}
				}
			}
	    };
	output.into()
}


#[proc_macro_derive(ValueUpdate)]
pub fn derive_value_update(input: TokenStream) -> TokenStream {
	let DeriveInput { ident, data, .. } = parse_macro_input!(input);
	let s = parse_struct!(data, ident);

	let mut tokens = Vec::new();
	let mut idents = Vec::new();
	for x in &s.fields {
		if let Some(ref ident) = x.ident {
			let token = match &x.ty {
				Type::Path(p) => {
					let path = &p.path;
					if let Some(src) = path.segments.span().unwrap().source_text() {
						let s = src.as_str().rsplit("::").next().unwrap_or(src.as_str());
						match s {
							"i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
								quote! {
									match value {
										base::value::RustPrimitiveValue::Integer(i) => {
											self.#ident = i.as_i128() as _;
											base::value::ValueUpdateResult::Success
										}
										v => base::value::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
								quote! {
									match value {
										base::value::RustPrimitiveValue::Integer(i) => {
											self.#ident = i.as_u128() as _;
											base::value::ValueUpdateResult::Success
										}
										v => base::value::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"f32" | "f64" => {
								quote! {
									match value {
										base::value::RustPrimitiveValue::Float(f) => {
											self.#ident = f.as_f64() as _;
											base::value::ValueUpdateResult::Success
										}
										v => base::value::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"String" => {
								quote! {
									match value {
										base::value::RustPrimitiveValue::String(i) => {
											self.#ident = i;
											base::value::ValueUpdateResult::Success
										}
										v => base::value::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"Vec<u8>" => {
								quote! {
									match value {
										base::value::RustPrimitiveValue::Bytes(i) => {
											self.#ident = i;
											base::value::ValueUpdateResult::Success
										}
										v => base::value::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							_ => {
								continue;
							}
						}
					} else {
						continue;
					}
				}
				_ => {
					Error::new(x.span().unwrap().into(), "Type is not supported")
						.into_compile_error()
				}
			};
			tokens.push(token);
			idents.push(ident);
		}
	}

	let output = quote! {
	        impl base::value::ValueUpdate for #ident {
				fn set_value(&mut self, key: &str, value: base::value::RustPrimitiveValue) -> base::value::ValueUpdateResult {
					match key {
						#( stringify!(#idents) => { #tokens }, )*
						_ => base::value::ValueUpdateResult::Invalid
					}
				}
			}
	    };
	output.into()
}


#[proc_macro_attribute]
pub fn filter_serialize(attr: TokenStream, input: TokenStream) -> TokenStream {
	let base = proc_macro2::TokenStream::from(input.clone());
	let DeriveInput { ident, data, .. } = parse_macro_input!(input);
	let _fields = attr.to_string();
	let fields = _fields.split(',').map(|it| it.trim()).collect::<Vec<_>>();

	let s = parse_struct!(data, ident);
	let mut types = Vec::new();
	let mut field_idents = Vec::new();
	for x in s.fields {
		if let Some(ident) = &x.ident {
			let field = ident.to_string();
			if fields.contains(&&*field) {
				let ty = &x.ty;
				let vis = &x.vis;
				let attr = &x.attrs;
				types.push(quote! {
					#(#attr)*
					#vis #ident: &'a #ty,
				});
				field_idents.push(Ident::new(&field, proc_macro2::Span::from(x.span().unwrap())));
			}
		}
	}
	let span = Span::call_site();
	let out_name = format_ident!("{}Ref",ident);
	let impl_block = quote_spanned! {span=>
		#[derive(serde::Serialize)]
		struct #out_name<'a> {
			#(#types)*
		}
		
		impl base::ser_ref::ToJsonValue for #ident {
			fn to_json(&self) -> serde_json::Value {
				serde_json::to_value(#out_name {
					#(#field_idents : &self.#field_idents,)*
				}).unwrap()
			}
		}
	};
	(quote! {
		#base
		#impl_block
	}).into()
}