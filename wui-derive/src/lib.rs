use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(WuiValue)]
pub fn derive_wui_value(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = input.ident;
	let fields = match input.data {
		Data::Struct(data) => data.fields,
		_ => {
			return syn::Error::new_spanned(name, "WuiValue can only be derived for structs")
				.to_compile_error()
				.into();
		}
	};

	let named = match fields {
		Fields::Named(named) => named.named,
		_ => {
			return syn::Error::new_spanned(name, "WuiValue requires named fields")
				.to_compile_error()
				.into();
		}
	};

	let entries = named.iter().map(|field| {
		let ident = field.ident.as_ref().unwrap();
		let key = ident.to_string();
		quote! {
			(#key.to_string(), wgui::wui::runtime::WuiValueConvert::to_wui_value(&self.#ident))
		}
	});

	let expanded = quote! {
		impl wgui::wui::runtime::WuiValueConvert for #name {
			fn to_wui_value(&self) -> wgui::wui::runtime::WuiValue {
				wgui::wui::runtime::WuiValue::object(vec![
					#(#entries),*
				])
			}
		}
	};

	expanded.into()
}
