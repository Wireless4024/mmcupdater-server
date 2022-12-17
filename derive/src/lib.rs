use proc_macro::TokenStream;

use quote::quote;
use syn::{Data, DeriveInput, Error, parse_macro_input, Type};
use syn::spanned::Spanned;

use base::{Integer, RustPrimitiveValue, ValueUpdate, ValueUpdateResult};

#[proc_macro_derive(ValueAccess)]
pub fn derive_value_access(input: TokenStream) -> TokenStream {
	let DeriveInput { ident, data, .. } = parse_macro_input!(input);
	let s = match data {
		Data::Struct(s) => { s }
		_ => {
			return Error::new(ident.span().unwrap().into(), "Not a struct")
				.into_compile_error()
				.into();
		}
	};
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
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::I8(self.#ident))}
							}
							"i16" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::I16(self.#ident))}
							}
							"i32" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::I32(self.#ident))}
							}
							"i64" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::I64(self.#ident))}
							}
							"i128" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::I128(self.#ident))}
							}
							"isize" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::ISize(self.#ident))}
							}
							"u8" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::U8(self.#ident))}
							}
							"u16" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::U16(self.#ident))}
							}
							"u32" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::U32(self.#ident))}
							}
							"u64" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::U64(self.#ident))}
							}
							"u128" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::U128(self.#ident))}
							}
							"usize" => {
								quote! {base::RustPrimitiveValueRef::Integer(base::Integer::USize(self.#ident))}
							}
							"f32" => {
								quote! {base::RustPrimitiveValueRef::Float(base::Float::F32(self.#ident))}
							}
							"f64" => {
								quote! {base::RustPrimitiveValueRef::Float(base::Float::F64(self.#ident))}
							}
							"String" => {
								quote! {base::RustPrimitiveValueRef::String(&self.#ident)}
							}
							"Vec<u8>" => {
								quote! {base::RustPrimitiveValueRef::Bytes(&self.#ident)}
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
	        impl base::ValueAccess for #ident {
				fn get_value(&self, key: &str) -> Option<base::RustPrimitiveValueRef> {
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
	let s = match data {
		Data::Struct(s) => { s }
		_ => {
			return Error::new(ident.span().unwrap().into(), "Not a struct")
				.into_compile_error()
				.into();
		}
	};

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
							"i8"|"i16" |"i32"|"i64"|"i128"|"isize" => {
								quote! {
									match value {
										base::RustPrimitiveValue::Integer(i) => {
											self.#ident = i.as_i128() as _;
											base::ValueUpdateResult::Success
										}
										v => base::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"u8"|"u16"|"u32"|"u64"|"u128"|"usize" => {
								quote! {
									match value {
										base::RustPrimitiveValue::Integer(i) => {
											self.#ident = i.as_u128() as _;
											base::ValueUpdateResult::Success
										}
										v => base::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"f32"|"f64" => {
								quote! {
									match value {
										base::RustPrimitiveValue::Float(f) => {
											self.#ident = f.as_f64() as _;
											base::ValueUpdateResult::Success
										}
										v => base::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"String" => {
								quote! {
									match value {
										base::RustPrimitiveValue::String(i) => {
											self.#ident = i;
											base::ValueUpdateResult::Success
										}
										v => base::ValueUpdateResult::TypeMismatch(v),
									}
								}
							}
							"Vec<u8>" => {
								quote! {
									match value {
										base::RustPrimitiveValue::Bytes(i) => {
											self.#ident = i;
											base::ValueUpdateResult::Success
										}
										v => base::ValueUpdateResult::TypeMismatch(v),
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
	        impl base::ValueUpdate for #ident {
				fn set_value(&mut self, key: &str, value: base::RustPrimitiveValue) -> base::ValueUpdateResult {
					match key {
						#( stringify!(#idents) => { #tokens }, )*
						_ => base::ValueUpdateResult::Invalid
					}
				}
			}
	    };
	output.into()
}