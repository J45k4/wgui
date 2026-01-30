use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
	parse_macro_input, Data, DeriveInput, Fields, FnArg, ImplItem, ItemImpl, ReturnType, Type,
};

#[proc_macro_derive(WuiModel)]
pub fn derive_wui_model(input: TokenStream) -> TokenStream {
	derive_wui_value_convert(input, "WuiModel")
}

fn derive_wui_value_convert(input: TokenStream, label: &str) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = input.ident;
	let fields = match input.data {
		Data::Struct(data) => data.fields,
		_ => {
			return syn::Error::new_spanned(name, format!("{label} can only be derived for structs"))
				.to_compile_error()
				.into();
		}
	};

	let named = match fields {
		Fields::Named(named) => named.named,
		_ => {
			return syn::Error::new_spanned(name, format!("{label} requires named fields"))
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

#[proc_macro_attribute]
pub fn wgui_controller(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let impl_block = parse_macro_input!(item as ItemImpl);
	match expand_wgui_controller(impl_block) {
		Ok(tokens) => tokens,
		Err(err) => err.to_compile_error().into(),
	}
}

enum HandlerArg {
	None,
	U32,
	I32,
	String,
}

struct HandlerMethod {
	ident: syn::Ident,
	arg: HandlerArg,
}

fn expand_wgui_controller(impl_block: ItemImpl) -> syn::Result<TokenStream> {
	let controller_ident = match *impl_block.self_ty.clone() {
		Type::Path(path) => path
			.path
			.segments
			.last()
			.map(|seg| seg.ident.clone())
			.ok_or_else(|| syn::Error::new_spanned(path, "wgui_controller requires a type name"))?,
		other => {
			return Err(syn::Error::new_spanned(
				other,
				"wgui_controller only supports impl blocks for named types",
			))
		}
	};

	let mut model_method: Option<(syn::Ident, Type)> = None;
	let mut handlers = Vec::new();

	for item in &impl_block.items {
		let ImplItem::Fn(method) = item else {
			continue;
		};

		let receiver = method.sig.inputs.first();
		let Some(FnArg::Receiver(recv)) = receiver else {
			continue;
		};

		let input_count = method
			.sig
			.inputs
			.iter()
			.filter(|arg| !matches!(arg, FnArg::Receiver(_)))
			.count();

		match (&recv.reference, &recv.mutability) {
			(Some(_), None) => {
				if input_count == 0 {
					if let ReturnType::Type(_, ty) = &method.sig.output {
						if matches!(**ty, Type::Tuple(_)) {
							continue;
						}
						if model_method.is_some() {
							return Err(syn::Error::new_spanned(
								&method.sig.ident,
								"wgui_controller requires exactly one &self method returning a model",
							));
						}
							model_method = Some((method.sig.ident.clone(), (**ty).clone()));
					}
				}
			}
			(Some(_), Some(_)) => {
				let arg = match input_count {
					0 => Some(HandlerArg::None),
					1 => method
						.sig
						.inputs
						.iter()
						.find_map(|arg| match arg {
							FnArg::Typed(pat) => Some(&*pat.ty),
							_ => None,
						})
						.and_then(handler_arg_from_type),
					_ => None,
				};
				if let Some(arg) = arg {
					handlers.push(HandlerMethod {
						ident: method.sig.ident.clone(),
						arg,
					});
				}
			}
			_ => {}
		}
	}

	let (model_method_ident, model_type) = model_method.ok_or_else(|| {
		syn::Error::new_spanned(
			&controller_ident,
			"wgui_controller requires an &self method that returns a model",
		)
	})?;

	let model_type_ident = match &model_type {
		Type::Path(path) => path
			.path
			.segments
			.last()
			.map(|seg| seg.ident.to_string())
			.ok_or_else(|| {
				syn::Error::new_spanned(model_type.clone(), "model type must be a named type")
			})?,
		_ => {
			return Err(syn::Error::new_spanned(
				model_type.clone(),
				"model type must be a named type",
			))
		}
	};

	let module_name = {
		let mut name = to_snake_case(&model_type_ident);
		if name.ends_with("_state") {
			name.truncate(name.len() - "_state".len());
		}
		name
	};

	let template_fn = format_ident!("__wgui_template_for_{}", controller_ident);
	let action_fn = format_ident!("__wgui_action_name_for_{}", controller_ident);
	let module_name_fn = format_ident!("__wgui_module_name_for_{}", controller_ident);

	let no_arg_handlers = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::None))
		.map(|handler| handler.ident.clone())
		.collect::<Vec<_>>();
	let u32_handlers = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::U32))
		.map(|handler| handler.ident.clone())
		.collect::<Vec<_>>();
	let i32_handlers = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::I32))
		.map(|handler| handler.ident.clone())
		.collect::<Vec<_>>();
	let string_handlers = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::String))
		.map(|handler| handler.ident.clone())
		.collect::<Vec<_>>();

	let no_arg_arms = no_arg_handlers.iter().map(|ident| {
		let name = ident.to_string();
		quote! { #name => { self.#ident(); true } }
	});
	let u32_arms = u32_handlers.iter().map(|ident| {
		let name = ident.to_string();
		quote! { #name => { self.#ident(arg); true } }
	});
	let i32_arms = i32_handlers.iter().map(|ident| {
		let name = ident.to_string();
		quote! { #name => { self.#ident(value); true } }
	});
	let string_arms = string_handlers
		.iter()
		.map(|ident| {
			let name = ident.to_string();
			quote! { #name => { self.#ident(value); true } }
		})
		.collect::<Vec<_>>();
	let string_arms_ref = &string_arms;

	let output = quote! {
		#impl_block

		#[allow(non_snake_case)]
		fn #module_name_fn() -> ::std::string::String {
			let fallback = #module_name;
			let path = ::std::path::Path::new(file!());
			let stem = path
				.file_stem()
				.and_then(|value| value.to_str())
				.unwrap_or("");
			let derived = if stem == "mod" {
				path.parent()
					.and_then(|parent| parent.file_name())
					.and_then(|value| value.to_str())
					.unwrap_or("")
			} else {
				stem
			};
			if derived.is_empty() {
				fallback.to_string()
			} else {
				derived.to_string()
			}
		}

	#[allow(non_snake_case)]
	fn #template_fn() -> &'static ::wgui::wui::runtime::Template {
		static TEMPLATE: ::std::sync::OnceLock<::wgui::wui::runtime::Template> = ::std::sync::OnceLock::new();
		TEMPLATE.get_or_init(|| {
			let module_name = #module_name_fn();
			let base_dir = ::std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/wui/pages"));
			let source_path = base_dir.join(format!("{}.wui", module_name));
			let source = ::std::fs::read_to_string(&source_path).unwrap_or_else(|err| {
				panic!("failed to read wui template {}: {}", source_path.display(), err)
			});
			::wgui::wui::runtime::Template::parse_with_dir(&source, &module_name, Some(base_dir))
				.unwrap_or_else(|diags| panic!("failed to parse wui template {}: {:?}", module_name, diags))
		})
	}

	#[allow(non_snake_case)]
	fn #action_fn(name: &str) -> ::std::string::String {
			let mut out = ::std::string::String::with_capacity(name.len());
			for (index, ch) in name.chars().enumerate() {
				if ch.is_uppercase() {
					if index != 0 {
						out.push('_');
					}
					for lower in ch.to_lowercase() {
						out.push(lower);
					}
				} else {
					out.push(ch);
				}
			}
			out
		}

	impl ::wgui::wui::runtime::WuiController for #controller_ident {
			fn render(&self) -> ::wgui::Item {
				let model = self.#model_method_ident();
				#template_fn().render(&model)
			}

			fn handle(&mut self, event: &::wgui::ClientEvent) -> bool {
				let Some(action) = #template_fn().decode(event) else {
					return false;
				};
				match action {
					::wgui::wui::runtime::RuntimeAction::Click { ref name, arg } => {
						let action_name = #action_fn(name);
						if let Some(arg) = arg {
							match action_name.as_str() {
								#(#u32_arms,)*
								_ => false,
							}
						} else {
							match action_name.as_str() {
								#(#no_arg_arms,)*
								_ => false,
							}
						}
					}
					::wgui::wui::runtime::RuntimeAction::TextChanged { ref name, value } => {
						let action_name = #action_fn(name);
						match action_name.as_str() {
							#(#string_arms_ref,)*
							_ => false,
						}
					}
					::wgui::wui::runtime::RuntimeAction::SliderChange { ref name, value } => {
						let action_name = #action_fn(name);
						match action_name.as_str() {
							#(#i32_arms,)*
							_ => false,
						}
					}
					::wgui::wui::runtime::RuntimeAction::Select { ref name, value } => {
						let action_name = #action_fn(name);
						match action_name.as_str() {
							#(#string_arms_ref,)*
							_ => false,
						}
					}
				}
			}
		}
	};

	Ok(output.into())
}

fn handler_arg_from_type(ty: &Type) -> Option<HandlerArg> {
	let Type::Path(path) = ty else {
		return None;
	};
	let ident = path.path.segments.last()?.ident.to_string();
	match ident.as_str() {
		"String" => Some(HandlerArg::String),
		"u32" => Some(HandlerArg::U32),
		"i32" => Some(HandlerArg::I32),
		_ => None,
	}
}

fn to_snake_case(value: &str) -> String {
	let mut out = String::with_capacity(value.len());
	for (index, ch) in value.chars().enumerate() {
		if ch.is_uppercase() {
			if index != 0 {
				out.push('_');
			}
			for lower in ch.to_lowercase() {
				out.push(lower);
			}
		} else {
			out.push(ch);
		}
	}
	out
}
