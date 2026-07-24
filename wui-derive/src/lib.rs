use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::path::{Path, PathBuf};
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input, Data, DeriveInput, Fields, FnArg, ImplItem, ItemFn, ItemImpl, LitStr, Pat,
	ReturnType, Signature, Token, Type,
};

#[proc_macro_derive(WguiModel)]
pub fn derive_wgui_model(input: TokenStream) -> TokenStream {
	derive_wui_value_convert(input, "WguiModel")
}

#[proc_macro_derive(WuiModel)]
pub fn derive_wui_model(input: TokenStream) -> TokenStream {
	derive_wui_value_convert(input, "WuiModel")
}

#[proc_macro_derive(Wdb)]
pub fn derive_wdb(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = input.ident;
	let fields = match input.data {
		Data::Struct(data) => data.fields,
		_ => {
			return syn::Error::new_spanned(name, "Wdb can only be derived for structs")
				.to_compile_error()
				.into();
		}
	};

	let named = match fields {
		Fields::Named(named) => named.named,
		_ => {
			return syn::Error::new_spanned(name, "Wdb requires named fields")
				.to_compile_error()
				.into();
		}
	};

	let model_schemas = named.iter().map(|field| {
		let ty = &field.ty;
		quote! { <#ty as wgui::wui::runtime::WdbModel>::schema() }
	});

	let expanded = quote! {
		impl wgui::wui::runtime::WdbSchema for #name {
			fn schema() -> ::std::vec::Vec<wgui::wui::runtime::WdbModelSchema> {
				vec![#(#model_schemas),*]
			}
		}
	};

	expanded.into()
}

fn derive_wui_value_convert(input: TokenStream, label: &str) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = input.ident;
	let fields = match input.data {
		Data::Struct(data) => data.fields,
		_ => {
			return syn::Error::new_spanned(
				name,
				format!("{label} can only be derived for structs"),
			)
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
	let schema_fields = named.iter().map(|field| {
		let ident = field.ident.as_ref().unwrap();
		let key = ident.to_string();
		let ty = &field.ty;
		quote! {
			wgui::wui::runtime::WdbFieldSchema {
				name: #key,
				rust_type: stringify!(#ty),
			}
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

		impl wgui::wui::runtime::WdbModel for #name {
			fn schema() -> wgui::wui::runtime::WdbModelSchema {
				wgui::wui::runtime::WdbModelSchema {
					model: stringify!(#name),
					fields: vec![#(#schema_fields),*],
				}
			}
		}
	};

	expanded.into()
}

#[proc_macro_attribute]
pub fn wgui_controller(attr: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attr as WguiControllerArgs);
	let impl_block = parse_macro_input!(item as ItemImpl);
	match expand_wgui_controller(args, impl_block) {
		Ok(tokens) => tokens,
		Err(err) => err.to_compile_error().into(),
	}
}

#[derive(Default)]
struct WguiControllerArgs {
	template: Option<String>,
	mode: TemplateMode,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum TemplateMode {
	#[default]
	Auto,
	Runtime,
	Compiled,
}

impl Parse for WguiControllerArgs {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let mut args = Self::default();
		while !input.is_empty() {
			let ident: syn::Ident = input.parse()?;
			input.parse::<Token![=]>()?;
			if ident == "template" {
				let value: LitStr = input.parse()?;
				args.template = Some(value.value());
			} else if ident == "mode" {
				let value: LitStr = input.parse()?;
				args.mode = match value.value().as_str() {
					"runtime" => TemplateMode::Runtime,
					"compiled" => TemplateMode::Compiled,
					"auto" => TemplateMode::Auto,
					other => {
						return Err(syn::Error::new_spanned(
							value,
							format!("unsupported wgui_controller mode {other:?}"),
						))
					}
				};
			} else {
				return Err(syn::Error::new_spanned(
					ident,
					"unsupported wgui_controller argument",
				));
			}
			if input.is_empty() {
				break;
			}
			input.parse::<Token![,]>()?;
		}
		Ok(args)
	}
}

enum HandlerArg {
	None,
	U32,
	I32,
	U32I32,
	String,
	Json,
}

struct HandlerMethod {
	ident: syn::Ident,
	arg: HandlerArg,
	is_async: bool,
}

struct FallbackEventHandler {
	ident: syn::Ident,
	is_async: bool,
}

struct HttpHandlerMethod {
	ident: syn::Ident,
	routes: Vec<String>,
	args: Vec<HttpHandlerArg>,
	is_async: bool,
}

enum HttpHandlerArg {
	Extractor { var: syn::Ident, ty: Box<Type> },
	Ctx { var: syn::Ident },
}

enum TitleReturn {
	String,
	OptionString,
}

fn expand_wgui_controller(
	args: WguiControllerArgs,
	mut impl_block: ItemImpl,
) -> syn::Result<TokenStream> {
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

	let mut state_method: Option<(syn::Ident, Type)> = None;
	let mut fallback_model_methods: Vec<(syn::Ident, Type)> = Vec::new();
	let mut title_method: Option<(syn::Ident, TitleReturn)> = None;
	let mut handlers = Vec::new();
	let mut fallback_event_handler: Option<FallbackEventHandler> = None;
	let mut process_method: Option<syn::Ident> = None;
	let mut http_handlers = Vec::new();

	for item in &mut impl_block.items {
		let ImplItem::Fn(method) = item else {
			continue;
		};

		let post_routes = take_wgui_post_routes(method)?;
		if !post_routes.is_empty() {
			http_handlers.push(http_handler_from_method(method, post_routes)?);
			continue;
		}

		if method.sig.ident == "process"
			&& method
				.sig
				.inputs
				.iter()
				.all(|arg| !matches!(arg, FnArg::Receiver(_)))
		{
			if process_method.is_some() {
				return Err(syn::Error::new_spanned(
					&method.sig.ident,
					"wgui_controller allows only one process method",
				));
			}
			process_method = Some(method.sig.ident.clone());
			continue;
		}

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
					if method.sig.asyncness.is_some() {
						continue;
					}
					if let ReturnType::Type(_, ty) = &method.sig.output {
						if matches!(**ty, Type::Tuple(_)) {
							continue;
						}
						if method.sig.ident == "title" {
							if title_method.is_some() {
								return Err(syn::Error::new_spanned(
									&method.sig.ident,
									"wgui_controller allows only one title method",
								));
							}
							title_method =
								Some((method.sig.ident.clone(), title_return_from_type(ty)?));
						} else if method.sig.ident == "state" {
							if state_method.is_some() {
								return Err(syn::Error::new_spanned(
									&method.sig.ident,
									"wgui_controller allows only one state method",
								));
							}
							state_method = Some((method.sig.ident.clone(), (**ty).clone()));
						} else {
							fallback_model_methods.push((method.sig.ident.clone(), (**ty).clone()));
						}
					}
				}
			}
			(Some(_), Some(_)) => {
				let arg_type = method.sig.inputs.iter().find_map(|arg| match arg {
					FnArg::Typed(pat) => Some(&*pat.ty),
					_ => None,
				});
				if input_count == 1 {
					if let Some(arg_type) = arg_type {
						if is_client_event_ref(arg_type) {
							if fallback_event_handler.is_some() {
								return Err(syn::Error::new_spanned(
									&method.sig.ident,
									"wgui_controller allows only one &mut self method that accepts &ClientEvent",
								));
							}
							fallback_event_handler = Some(FallbackEventHandler {
								ident: method.sig.ident.clone(),
								is_async: method.sig.asyncness.is_some(),
							});
							continue;
						}
					}
				}

				let arg = match input_count {
					0 => Some(HandlerArg::None),
					1 => arg_type.and_then(handler_arg_from_type),
					2 => handler_two_args_from_method(method),
					_ => None,
				};
				if let Some(arg) = arg {
					handlers.push(HandlerMethod {
						ident: method.sig.ident.clone(),
						arg,
						is_async: method.sig.asyncness.is_some(),
					});
				}
			}
			_ => {}
		}
	}

	let (model_method_ident, model_type) = if let Some(state_method) = state_method {
		state_method
	} else {
		if fallback_model_methods.len() > 1 {
			return Err(syn::Error::new_spanned(
				&controller_ident,
				"wgui_controller requires exactly one &self method returning a model, or a method named state",
			));
		}
		fallback_model_methods.pop().ok_or_else(|| {
			syn::Error::new_spanned(
				&controller_ident,
				"wgui_controller requires an &self method that returns a model",
			)
		})?
	};

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
	let direct_item_render = model_method_ident == "render" && model_type_ident == "Item";

	let module_name = {
		let mut name = to_snake_case(&model_type_ident);
		if name.ends_with("_state") {
			name.truncate(name.len() - "_state".len());
		}
		name
	};

	let explicit_template = if let Some(template) = args.template.as_ref() {
		quote! { ::std::option::Option::Some(#template.to_string()) }
	} else {
		quote! { ::std::option::Option::None }
	};
	let template_fn = format_ident!("__wgui_template_for_{}", controller_ident);
	let action_fn = format_ident!("__wgui_action_name_for_{}", controller_ident);
	let module_name_fn = format_ident!("__wgui_module_name_for_{}", controller_ident);
	let template_impl = if direct_item_render {
		quote! {}
	} else {
		template_impl_tokens(
			&args,
			&controller_ident.to_string(),
			&module_name,
			&module_name_fn,
			&template_fn,
		)?
	};
	let title_impl = title_method.map(|(ident, return_type)| match return_type {
		TitleReturn::String => quote! {
			fn title(&self) -> ::std::option::Option<::std::string::String> {
				::std::option::Option::Some(self.#ident())
			}
		},
		TitleReturn::OptionString => quote! {
			fn title(&self) -> ::std::option::Option<::std::string::String> {
				self.#ident()
			}
		},
	});
	let process_impl = process_method.as_ref().map(|ident| {
		quote! {
			async fn process(
				ctx: ::wgui::wui::runtime::ControllerProcessCtx,
			) -> ::wgui::wui::runtime::anyhow::Result<()>
			where
				Self: Sized,
			{
				Self::#ident(ctx).await;
				::std::result::Result::Ok(())
			}
		}
	});
	let http_impl = http_impl_tokens(&http_handlers);

	let no_arg_arms = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::None))
		.map(|handler| {
			let ident = &handler.ident;
			let name = ident.to_string();
			if handler.is_async {
				quote! { #name => { self.#ident().await; true } }
			} else {
				quote! { #name => { self.#ident(); true } }
			}
		});
	let u32_arms = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::U32))
		.map(|handler| {
			let ident = &handler.ident;
			let name = ident.to_string();
			if handler.is_async {
				quote! { #name => { self.#ident(arg).await; true } }
			} else {
				quote! { #name => { self.#ident(arg); true } }
			}
		});
	let i32_arms = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::I32))
		.map(|handler| {
			let ident = &handler.ident;
			let name = ident.to_string();
			if handler.is_async {
				quote! { #name => { self.#ident(value).await; true } }
			} else {
				quote! { #name => { self.#ident(value); true } }
			}
		});
	let u32_i32_arms = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::U32I32))
		.map(|handler| {
			let ident = &handler.ident;
			let name = ident.to_string();
			if handler.is_async {
				quote! { #name => { self.#ident(arg, value).await; true } }
			} else {
				quote! { #name => { self.#ident(arg, value); true } }
			}
		});
	let string_arms = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::String))
		.map(|handler| {
			let ident = &handler.ident;
			let name = ident.to_string();
			if handler.is_async {
				quote! { #name => { self.#ident(value).await; true } }
			} else {
				quote! { #name => { self.#ident(value); true } }
			}
		})
		.collect::<Vec<_>>();
	let string_arms_ref = &string_arms;
	let json_arms = handlers
		.iter()
		.filter(|handler| matches!(handler.arg, HandlerArg::Json))
		.map(|handler| {
			let ident = &handler.ident;
			let name = ident.to_string();
			if handler.is_async {
				quote! { #name => { self.#ident(payload).await; true } }
			} else {
				quote! { #name => { self.#ident(payload); true } }
			}
		})
		.collect::<Vec<_>>();
	let json_arms_ref = &json_arms;
	let fallback_decode = if let Some(handler) = &fallback_event_handler {
		let ident = &handler.ident;
		if handler.is_async {
			quote! {
				return self.#ident(event).await;
			}
		} else {
			quote! {
				return self.#ident(event);
			}
		}
	} else {
		quote! {
			return false;
		}
	};

	if direct_item_render {
		let output = quote! {
			#impl_block

			#[::wgui::wui::runtime::async_trait]
			impl ::wgui::wui::runtime::WuiController for #controller_ident {
				fn render(&self) -> ::wgui::Item {
					self.#model_method_ident()
				}

				#title_impl
				#http_impl
				#process_impl

				async fn handle(&mut self, event: &::wgui::ClientEvent) -> bool {
					#fallback_decode
				}
			}
		};

		return Ok(output.into());
	}

	let output = quote! {
		#impl_block

		#[allow(non_snake_case)]
		fn #module_name_fn() -> ::std::vec::Vec<::std::string::String> {
			if let ::std::option::Option::Some(explicit) = #explicit_template {
				return ::std::vec![explicit];
			}

			let fallback = #module_name;
			let path = ::std::path::Path::new(file!());
			let stem = path
				.file_stem()
				.and_then(|value| value.to_str())
				.unwrap_or("");

			let old_derived = if stem == "mod" {
				path.parent()
					.and_then(|parent| parent.file_name())
					.and_then(|value| value.to_str())
					.unwrap_or("")
			} else {
				stem
			};

			let mut candidates = ::std::vec::Vec::new();
			let parts = path
				.components()
				.filter_map(|component| match component {
					::std::path::Component::Normal(value) => value.to_str(),
					_ => ::std::option::Option::None,
				})
				.collect::<::std::vec::Vec<_>>();
			if let ::std::option::Option::Some(src_index) = parts.iter().rposition(|part| *part == "src") {
				let mut module_parts = parts
					.iter()
					.skip(src_index + 1)
					.map(|part| (*part).to_string())
					.collect::<::std::vec::Vec<_>>();
				if let ::std::option::Option::Some(last) = module_parts.last_mut() {
					if let ::std::option::Option::Some(stripped) = last.strip_suffix(".rs") {
						*last = stripped.to_string();
					}
				}
				if module_parts.last().map(|part| part == "mod").unwrap_or(false) {
					module_parts.pop();
				}
				if !module_parts.is_empty() {
					candidates.push(module_parts.join("/"));
				}
			}
			if !old_derived.is_empty() {
				candidates.push(old_derived.to_string());
			}
			candidates.push(fallback.to_string());

			let mut unique = ::std::vec::Vec::new();
			for candidate in candidates {
				if !unique.iter().any(|existing| existing == &candidate) {
					unique.push(candidate);
				}
			}
			unique
		}

	#template_impl

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

	#[::wgui::wui::runtime::async_trait]
	impl ::wgui::wui::runtime::WuiController for #controller_ident {
		fn render(&self) -> ::wgui::Item {
			let model = self.#model_method_ident();
			#template_fn().render(&model)
		}

		fn render_with_path(&self, path: &str) -> ::wgui::Item {
			let model = self.#model_method_ident();
			#template_fn().render_with_path(&model, path)
		}

		fn render_with_route(
			&self,
			route: &::wgui::wui::runtime::RouteContext,
		) -> ::wgui::Item {
			let model = self.#model_method_ident();
			#template_fn().render_with_route(&model, route)
		}

		#title_impl
		#http_impl
		#process_impl

		fn route_title(&self, path: &str) -> ::std::option::Option<::std::string::String> {
			#template_fn().title_for_path(path)
		}

		async fn handle(&mut self, event: &::wgui::ClientEvent) -> bool {
			let Some(action) = #template_fn().decode(event) else {
				#fallback_decode
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
					::wgui::wui::runtime::RuntimeAction::SliderChange { ref name, arg, value } => {
						let action_name = #action_fn(name);
						if let Some(arg) = arg {
							match action_name.as_str() {
								#(#u32_i32_arms,)*
								_ => false,
							}
						} else {
							match action_name.as_str() {
								#(#i32_arms,)*
								_ => false,
							}
						}
					}
					::wgui::wui::runtime::RuntimeAction::Select { ref name, value } => {
						let action_name = #action_fn(name);
						match action_name.as_str() {
							#(#string_arms_ref,)*
							_ => false,
						}
					}
					::wgui::wui::runtime::RuntimeAction::Custom { ref name, payload, .. } => {
						let action_name = #action_fn(name);
						match action_name.as_str() {
							#(#json_arms_ref,)*
							_ => false,
						}
					}
				}
			}
		}
	};

	Ok(output.into())
}

fn take_wgui_post_routes(method: &mut syn::ImplItemFn) -> syn::Result<Vec<String>> {
	let mut routes = Vec::new();
	let mut attrs = Vec::new();
	for attr in method.attrs.drain(..) {
		if attr.path().is_ident("wgui_post") {
			let route: LitStr = attr.parse_args()?;
			routes.push(route.value());
		} else {
			attrs.push(attr);
		}
	}
	method.attrs = attrs;
	Ok(routes)
}

fn http_handler_from_method(
	method: &syn::ImplItemFn,
	routes: Vec<String>,
) -> syn::Result<HttpHandlerMethod> {
	let receiver = method.sig.inputs.first();
	if !matches!(
		receiver,
		Some(FnArg::Receiver(recv)) if recv.reference.is_some() && recv.mutability.is_some()
	) {
		return Err(syn::Error::new_spanned(
			&method.sig.ident,
			"wgui_post handlers must use &mut self",
		));
	}

	let mut args = Vec::new();
	for (index, arg) in method
		.sig
		.inputs
		.iter()
		.filter_map(|arg| match arg {
			FnArg::Typed(pat) => Some(&*pat.ty),
			_ => None,
		})
		.enumerate()
	{
		let var = format_ident!("__wgui_http_arg_{index}");
		if is_http_ctx_type(arg) {
			args.push(HttpHandlerArg::Ctx { var });
		} else {
			args.push(HttpHandlerArg::Extractor {
				var,
				ty: Box::new((*arg).clone()),
			});
		}
	}

	Ok(HttpHandlerMethod {
		ident: method.sig.ident.clone(),
		routes,
		args,
		is_async: method.sig.asyncness.is_some(),
	})
}

fn http_impl_tokens(handlers: &[HttpHandlerMethod]) -> proc_macro2::TokenStream {
	if handlers.is_empty() {
		return quote! {};
	}

	let route_specs = handlers.iter().flat_map(|handler| {
		let ident = &handler.ident;
		handler.routes.iter().map(move |route| {
			quote! {
				::wgui::HttpRouteSpec {
					method: "POST",
					path: #route,
					id: concat!(#route, "#", stringify!(#ident)),
				}
			}
		})
	});
	let route_arms = handlers.iter().flat_map(|handler| {
		let ident = &handler.ident;
		handler.routes.iter().map(move |route| {
			let route_id = quote! { concat!(#route, "#", stringify!(#ident)) };
			let extractors = handler.args.iter().map(|arg| match arg {
				HttpHandlerArg::Extractor { var, ty } => {
					let ty = ty.as_ref();
					quote! {
						let #var: #ty = match <#ty as ::wgui::FromHttpRequest>::from_http_request(&request) {
							::std::result::Result::Ok(value) => value,
							::std::result::Result::Err(response) => {
								return ::std::option::Option::Some(response);
							}
						};
					}
				}
				HttpHandlerArg::Ctx { var } => quote! {
					let #var = ctx.clone();
				},
			});
			let call_args = handler.args.iter().map(|arg| match arg {
				HttpHandlerArg::Extractor { var, .. } | HttpHandlerArg::Ctx { var } => var,
			});
			if handler.is_async {
				quote! {
					#route_id => {
						#(#extractors)*
						::std::option::Option::Some(self.#ident(#(#call_args),*).await)
					}
				}
			} else {
				quote! {
					#route_id => {
						#(#extractors)*
						::std::option::Option::Some(self.#ident(#(#call_args),*))
					}
				}
			}
		})
	});

	quote! {
		fn http_routes() -> ::std::vec::Vec<::wgui::HttpRouteSpec>
		where
			Self: Sized,
		{
			::std::vec![#(#route_specs),*]
		}

		async fn handle_http(
			&mut self,
			route: &str,
			request: ::wgui::HttpRequest,
			ctx: ::wgui::HttpCtx,
		) -> ::std::option::Option<::wgui::HttpResponse>
		where
			Self: Sized,
		{
			match route {
				#(#route_arms,)*
				_ => ::std::option::Option::None,
			}
		}
	}
}

fn template_impl_tokens(
	args: &WguiControllerArgs,
	controller_name: &str,
	fallback_module_name: &str,
	module_name_fn: &proc_macro2::Ident,
	template_fn: &proc_macro2::Ident,
) -> syn::Result<proc_macro2::TokenStream> {
	let runtime_impl = runtime_template_impl(module_name_fn, template_fn, None);
	match args.mode {
		TemplateMode::Runtime => Ok(runtime_impl),
		TemplateMode::Compiled => compiled_template_impl(
			args,
			controller_name,
			fallback_module_name,
			template_fn,
			None,
		),
		TemplateMode::Auto => {
			let runtime_impl = runtime_template_impl(
				module_name_fn,
				template_fn,
				Some(quote! { #[cfg(debug_assertions)] }),
			);
			let compiled_impl = compiled_template_impl(
				args,
				controller_name,
				fallback_module_name,
				template_fn,
				Some(quote! { #[cfg(not(debug_assertions))] }),
			)?;
			Ok(quote! {
				#runtime_impl
				#compiled_impl
			})
		}
	}
}

fn runtime_template_impl(
	module_name_fn: &proc_macro2::Ident,
	template_fn: &proc_macro2::Ident,
	cfg_attr: Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
	let cfg_attr = cfg_attr.unwrap_or_default();
	quote! {
		#cfg_attr
		#[allow(non_snake_case)]
		fn #template_fn() -> &'static ::wgui::wui::runtime::Template {
			static TEMPLATE: ::std::sync::OnceLock<::wgui::wui::runtime::Template> = ::std::sync::OnceLock::new();
			TEMPLATE.get_or_init(|| {
				let base_dir = ::std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/wui"));
				let candidates = #module_name_fn();
				let mut read_errors = ::std::vec::Vec::new();
				for module_name in candidates {
					let source_path = base_dir.join(format!("{}.wui", module_name));
					let source = match ::std::fs::read_to_string(&source_path) {
						::std::result::Result::Ok(source) => source,
						::std::result::Result::Err(err) => {
							read_errors.push(format!("{}: {}", source_path.display(), err));
							continue;
						}
					};
					return ::wgui::wui::runtime::Template::parse_with_dir(&source, &module_name, source_path.parent())
						.unwrap_or_else(|diags| panic!("failed to parse wui template {}: {:?}", module_name, diags));
				}
				panic!("failed to read wui template; tried {}", read_errors.join(", "))
			})
		}
	}
}

fn compiled_template_impl(
	args: &WguiControllerArgs,
	controller_name: &str,
	fallback_module_name: &str,
	template_fn: &proc_macro2::Ident,
	cfg_attr: Option<proc_macro2::TokenStream>,
) -> syn::Result<proc_macro2::TokenStream> {
	let compiled = read_compiled_template(args, controller_name, fallback_module_name)?;
	let cfg_attr = cfg_attr.unwrap_or_default();
	let module_name = compiled.module_name;
	let root_path = compiled.root_path;
	let root_source = compiled.root_source;
	let sources = compiled.sources.iter().map(|(path, source)| {
		quote! { (#path, #source) }
	});
	Ok(quote! {
		#cfg_attr
		#[allow(non_snake_case)]
		fn #template_fn() -> &'static ::wgui::wui::runtime::Template {
			static TEMPLATE: ::std::sync::OnceLock<::wgui::wui::runtime::Template> = ::std::sync::OnceLock::new();
			TEMPLATE.get_or_init(|| {
				const SOURCES: &[(&str, &str)] = &[
					#(#sources),*
				];
				let source_path = ::std::path::Path::new(#root_path);
				::wgui::wui::runtime::Template::parse_with_sources(
					#root_source,
					#module_name,
					source_path.parent(),
					SOURCES,
				)
				.unwrap_or_else(|diags| panic!("failed to parse compiled wui template {}: {:?}", #module_name, diags))
			})
		}
	})
}

struct CompiledTemplate {
	module_name: String,
	root_path: String,
	root_source: String,
	sources: Vec<(String, String)>,
}

fn read_compiled_template(
	args: &WguiControllerArgs,
	controller_name: &str,
	fallback_module_name: &str,
) -> syn::Result<CompiledTemplate> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
		.map(PathBuf::from)
		.map_err(|err| syn::Error::new(proc_macro2::Span::call_site(), err))?;
	let base_dir = manifest_dir.join("wui");
	let candidates = template_candidates(
		args.template.as_deref(),
		&base_dir,
		controller_name,
		fallback_module_name,
	);
	let mut read_errors = Vec::new();
	for module_name in candidates {
		let source_path = base_dir.join(format!("{module_name}.wui"));
		let source = match std::fs::read_to_string(&source_path) {
			Ok(source) => source,
			Err(err) => {
				read_errors.push(format!("{}: {}", source_path.display(), err));
				continue;
			}
		};
		let source_path = normalize_template_path(&source_path);
		let generated =
			wui_core::compiler::compile_with_dir(&source, &module_name, source_path.parent())
				.map_err(|diags| template_diagnostics_error(&source_path, &diags))?;
		let mut sources = vec![(source_path.display().to_string(), source.clone())];
		for path in generated.source_files() {
			let path = normalize_template_path(&path);
			let import_source = std::fs::read_to_string(&path).map_err(|err| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					format!("failed to read WUI import {}: {err}", path.display()),
				)
			})?;
			sources.push((path.display().to_string(), import_source));
		}
		sources.sort_by(|a, b| a.0.cmp(&b.0));
		sources.dedup_by(|a, b| a.0 == b.0);
		return Ok(CompiledTemplate {
			module_name,
			root_path: source_path.display().to_string(),
			root_source: source,
			sources,
		});
	}
	Err(syn::Error::new(
		proc_macro2::Span::call_site(),
		format!(
			"failed to read wui template; tried {}",
			read_errors.join(", ")
		),
	))
}

fn template_candidates(
	explicit_template: Option<&str>,
	base_dir: &Path,
	controller_name: &str,
	fallback_module_name: &str,
) -> Vec<String> {
	if let Some(template) = explicit_template {
		return vec![template.to_string()];
	}

	let mut candidates = Vec::new();
	let controller_derived = controller_template_name(controller_name);
	if !controller_derived.is_empty() {
		candidates.push(controller_derived.clone());
		candidates.extend(wui_files_matching_stem(base_dir, &controller_derived));
	}
	candidates.push(fallback_module_name.to_string());
	candidates.extend(wui_files_matching_stem(base_dir, fallback_module_name));

	let mut unique = Vec::new();
	for candidate in candidates {
		if !unique.iter().any(|existing| existing == &candidate) {
			unique.push(candidate);
		}
	}
	unique
}

fn controller_template_name(controller_name: &str) -> String {
	let stripped = controller_name
		.strip_suffix("Controller")
		.unwrap_or(controller_name);
	to_snake_case(stripped)
}

fn wui_files_matching_stem(base_dir: &Path, stem: &str) -> Vec<String> {
	let mut files = Vec::new();
	collect_matching_wui_files(base_dir, base_dir, stem, &mut files);
	files.sort();
	files
}

fn collect_matching_wui_files(base_dir: &Path, dir: &Path, stem: &str, out: &mut Vec<String>) {
	let Ok(entries) = std::fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		let Ok(file_type) = entry.file_type() else {
			continue;
		};
		if file_type.is_dir() {
			collect_matching_wui_files(base_dir, &path, stem, out);
			continue;
		}
		if path.extension().and_then(|ext| ext.to_str()) != Some("wui") {
			continue;
		}
		if path.file_stem().and_then(|value| value.to_str()) != Some(stem) {
			continue;
		}
		if let Ok(relative) = path.strip_prefix(base_dir) {
			let mut without_ext = relative.to_path_buf();
			without_ext.set_extension("");
			let candidate = without_ext
				.components()
				.filter_map(|component| match component {
					std::path::Component::Normal(value) => value.to_str(),
					_ => None,
				})
				.collect::<Vec<_>>()
				.join("/");
			if !candidate.is_empty() {
				out.push(candidate);
			}
		}
	}
}

fn normalize_template_path(path: &Path) -> PathBuf {
	std::fs::canonicalize(path).unwrap_or_else(|_| {
		let mut out = PathBuf::new();
		for component in path.components() {
			match component {
				std::path::Component::CurDir => {}
				std::path::Component::ParentDir => {
					out.pop();
				}
				_ => out.push(component.as_os_str()),
			}
		}
		out
	})
}

fn template_diagnostics_error(
	path: &Path,
	diags: &[wui_core::diagnostic::Diagnostic],
) -> syn::Error {
	let details = diags
		.iter()
		.map(|diag| {
			format!(
				"{}:{}-{}: {}",
				path.display(),
				diag.span.start,
				diag.span.end,
				diag.message
			)
		})
		.collect::<Vec<_>>()
		.join("\n");
	syn::Error::new(
		proc_macro2::Span::call_site(),
		format!("failed to compile WUI template:\n{details}"),
	)
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
		"Value" if is_serde_json_value_path(path) => Some(HandlerArg::Json),
		_ => None,
	}
}

fn is_http_ctx_type(ty: &Type) -> bool {
	let Type::Path(path) = ty else {
		return false;
	};
	let segments = path
		.path
		.segments
		.iter()
		.map(|segment| segment.ident.to_string())
		.collect::<Vec<_>>();
	segments == ["HttpCtx"] || segments == ["wgui", "HttpCtx"]
}

fn is_serde_json_value_path(path: &syn::TypePath) -> bool {
	let segments = path
		.path
		.segments
		.iter()
		.map(|segment| segment.ident.to_string())
		.collect::<Vec<_>>();
	segments == ["serde_json", "Value"] || segments == ["wgui", "serde_json", "Value"]
}

fn handler_two_args_from_method(method: &syn::ImplItemFn) -> Option<HandlerArg> {
	let mut types = method.sig.inputs.iter().filter_map(|arg| match arg {
		FnArg::Typed(pat) => Some(&*pat.ty),
		_ => None,
	});
	let first = types.next()?;
	let second = types.next()?;
	if matches!(handler_arg_from_type(first), Some(HandlerArg::U32))
		&& matches!(handler_arg_from_type(second), Some(HandlerArg::I32))
	{
		Some(HandlerArg::U32I32)
	} else {
		None
	}
}

fn title_return_from_type(ty: &Type) -> syn::Result<TitleReturn> {
	let Type::Path(path) = ty else {
		return Err(syn::Error::new_spanned(
			ty,
			"title must return String or Option<String>",
		));
	};
	let Some(segment) = path.path.segments.last() else {
		return Err(syn::Error::new_spanned(
			ty,
			"title must return String or Option<String>",
		));
	};
	if segment.ident == "String" {
		return Ok(TitleReturn::String);
	}
	if segment.ident != "Option" {
		return Err(syn::Error::new_spanned(
			ty,
			"title must return String or Option<String>",
		));
	}
	let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
		return Err(syn::Error::new_spanned(
			ty,
			"title must return String or Option<String>",
		));
	};
	let Some(syn::GenericArgument::Type(Type::Path(inner))) = args.args.first() else {
		return Err(syn::Error::new_spanned(
			ty,
			"title must return String or Option<String>",
		));
	};
	let is_string = inner
		.path
		.segments
		.last()
		.map(|segment| segment.ident == "String")
		.unwrap_or(false);
	if is_string {
		Ok(TitleReturn::OptionString)
	} else {
		Err(syn::Error::new_spanned(
			ty,
			"title must return String or Option<String>",
		))
	}
}

fn is_client_event_ref(ty: &Type) -> bool {
	let Type::Reference(reference) = ty else {
		return false;
	};
	let Type::Path(path) = reference.elem.as_ref() else {
		return false;
	};
	path.path
		.segments
		.last()
		.map(|segment| segment.ident == "ClientEvent")
		.unwrap_or(false)
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

// ============================================================================
// #[route] attribute macro — see plans.md Phase 4.
//
// Generates a sibling marker struct implementing `RouteHandler` and a
// `pub const <fn>_route` handle that users pass to `Wgui::add_route`.
//
//     #[route("/todos/:id", method = "POST")]
//     fn action_toggle(ctx: &Ctx<AppState>, id: u32) -> Redirect { ... }
//
// expands to:
//
//     fn action_toggle(ctx: &Ctx<AppState>, id: u32) -> Redirect { ... }
//     struct __ActionToggleRoute;
//     impl RouteHandler for __ActionToggleRoute {
//         type State = AppState;
//         fn path(&self) -> &str { "/todos/:id" }
//         fn method(&self) -> HttpMethod { HttpMethod::Post }
//         fn call(self, ctx: &Ctx<AppState>, params: PathParams) -> RouteFuture {
//             let id: u32 = match params.get::<u32>("id") {
//                 Some(Ok(v)) => v,
//                 _ => return Box::pin(async { RouteResult::NotFound }),
//             };
//             Box::pin(async move {
//                 let __result = action_toggle(ctx, id).await;
//                 RouteResult::from(__result)
//             })
//         }
//     }
//     pub(crate) const action_toggle_route: __ActionToggleRoute = __ActionToggleRoute;
// ============================================================================

#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attr as RouteArgs);
	let item_fn = parse_macro_input!(item as ItemFn);
	match expand_route(args, item_fn, "route") {
		Ok(tokens) => tokens.into(),
		Err(err) => err.to_compile_error().into(),
	}
}

/// Construct a WUI-backed `View` from a model expression or anonymous object.
/// The route dispatcher supplies the template selected by `#[route(..., view)]`.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ViewMacroInput);
	let model = match input {
		ViewMacroInput::Object(entries) => view_object_tokens(&entries),
		ViewMacroInput::Expr(expr) => quote! {
			::wgui::wui::runtime::WuiValueConvert::to_wui_value(&(#expr))
		},
	};
	quote! {
		::wgui::View::wui(#model)
	}
	.into()
}

enum ViewMacroInput {
	Object(Vec<(syn::Ident, ViewMacroValue)>),
	Expr(syn::Expr),
}

enum ViewMacroValue {
	Object(Vec<(syn::Ident, ViewMacroValue)>),
	Expr(syn::Expr),
}

impl Parse for ViewMacroInput {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		if input.peek(syn::token::Brace) {
			return Ok(Self::Object(parse_view_object(input)?));
		}
		Ok(Self::Expr(input.parse()?))
	}
}

fn parse_view_object(input: ParseStream<'_>) -> syn::Result<Vec<(syn::Ident, ViewMacroValue)>> {
	let content;
	syn::braced!(content in input);
	let mut entries = Vec::new();
	while !content.is_empty() {
		let key: syn::Ident = content.parse()?;
		content.parse::<Token![:]>()?;
		let value = if content.peek(syn::token::Brace) {
			ViewMacroValue::Object(parse_view_object(&content)?)
		} else {
			ViewMacroValue::Expr(content.parse()?)
		};
		entries.push((key, value));
		if content.is_empty() {
			break;
		}
		content.parse::<Token![,]>()?;
	}
	Ok(entries)
}

fn view_object_tokens(entries: &[(syn::Ident, ViewMacroValue)]) -> TokenStream2 {
	let entries = entries.iter().map(|(key, value)| {
		let key = key.to_string();
		let value = view_value_tokens(value);
		quote! { (::std::string::String::from(#key), #value) }
	});
	quote! {
		::wgui::wui::runtime::WuiValue::object(::std::vec![#(#entries),*])
	}
}

fn view_value_tokens(value: &ViewMacroValue) -> TokenStream2 {
	match value {
		ViewMacroValue::Object(entries) => view_object_tokens(entries),
		ViewMacroValue::Expr(expr) => quote! {
			::wgui::wui::runtime::WuiValueConvert::to_wui_value(&(#expr))
		},
	}
}

/// Declare a re-renderable partial route. The generated `*_partial` handle
/// is registered with `Wgui::add_partial`.
#[proc_macro_attribute]
pub fn partial(attr: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attr as RouteArgs);
	if !matches!(args.method, RouteMethod::Get) {
		return syn::Error::new_spanned(
			proc_macro2::Literal::string(&args.path),
			"#[partial] only supports GET-style rendering",
		)
		.to_compile_error()
		.into();
	}
	let item_fn = parse_macro_input!(item as ItemFn);
	match expand_route(args, item_fn, "partial") {
		Ok(tokens) => tokens.into(),
		Err(err) => err.to_compile_error().into(),
	}
}

struct RouteArgs {
	path: String,
	method: RouteMethod,
	view: bool,
	template: Option<String>,
}

#[derive(Default)]
enum RouteMethod {
	#[default]
	Get,
	Post,
}

impl Parse for RouteArgs {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let path: LitStr = input.parse()?;
		let mut method = RouteMethod::default();
		let mut view = false;
		let mut template = None;
		while !input.is_empty() {
			input.parse::<Token![,]>()?;
			if input.is_empty() {
				break;
			}
			let ident: syn::Ident = input.parse()?;
			if ident == "view" {
				view = true;
				continue;
			}
			input.parse::<Token![=]>()?;
			let val: LitStr = input.parse()?;
			if ident == "method" {
				method = match val.value().as_str() {
					"GET" | "get" => RouteMethod::Get,
					"POST" | "post" => RouteMethod::Post,
					other => {
						return Err(syn::Error::new_spanned(
							val,
							format!("unsupported route method {other:?}; use GET or POST"),
						))
					}
				};
			} else if ident == "template" {
				view = true;
				template = Some(val.value());
			} else {
				return Err(syn::Error::new_spanned(
					ident,
					"unsupported #[route] argument; only `method = \"GET\"|\"POST\"` is recognized",
				));
			}
		}
		Ok(RouteArgs {
			path: path.value(),
			method,
			view,
			template,
		})
	}
}

fn expand_route(args: RouteArgs, item_fn: ItemFn, handle_kind: &str) -> syn::Result<TokenStream2> {
	if args.view && !matches!(args.method, RouteMethod::Get) {
		return Err(syn::Error::new_spanned(
			&item_fn.sig.ident,
			"the `view` route option only supports GET handlers",
		));
	}
	let fn_ident = item_fn.sig.ident.clone();
	let fn_is_async = item_fn.sig.asyncness.is_some();
	let return_type = match &item_fn.sig.output {
		ReturnType::Default => quote! { () },
		ReturnType::Type(_, ty) => quote! { #ty },
	};

	let (ctx_ident, state_type) = extract_ctx_arg(&item_fn.sig)?;
	let path_param_names = path_param_names(&args.path);
	let param_args = extract_param_args(&item_fn.sig, &path_param_names, &args.method)?;

	let marker_ident = marker_ident(&fn_ident);
	let route_const_ident = route_const_ident(&fn_ident, handle_kind);

	let path_lit = &args.path;
	let template_impl = if args.view {
		let template_name = args
			.template
			.clone()
			.unwrap_or_else(|| standard_route_template(&args.path));
		let template_fn = format_ident!("__wgui_template_for_{}", fn_ident);
		quote! {
			fn #template_fn() -> &'static wgui::wui::runtime::Template {
				static TEMPLATE: ::std::sync::OnceLock<wgui::wui::runtime::Template> = ::std::sync::OnceLock::new();
				TEMPLATE.get_or_init(|| {
					let source_path = ::std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/wui"))
						.join(format!("{}.wui", #template_name));
					let source = ::std::fs::read_to_string(&source_path).unwrap_or_else(|err| {
						panic!("failed to read WUI template {}: {}", source_path.display(), err)
					});
					wgui::wui::runtime::Template::parse_with_dir(&source, #template_name, source_path.parent())
						.unwrap_or_else(|diags| panic!("failed to parse WUI template {}: {:?}", #template_name, diags))
				})
			}
		}
	} else {
		quote! {}
	};
	let template_method = if args.view {
		let template_fn = format_ident!("__wgui_template_for_{}", fn_ident);
		quote! {
			fn wui_template(&self) -> ::std::option::Option<&'static wgui::wui::runtime::Template> {
				::std::option::Option::Some(#template_fn())
			}
		}
	} else {
		quote! {}
	};
	let method_arm = match args.method {
		RouteMethod::Get => quote! { wgui::wui::route_handler::HttpMethod::Get },
		RouteMethod::Post => quote! { wgui::wui::route_handler::HttpMethod::Post },
	};

	// Bind path params by extracting from `PathParams` using their ident as
	// the lookup key. Each extracted value is bound with the user's
	// original ident so the call site reads naturally.
	let param_bindings = param_args.iter().map(|arg| match arg {
		RouteArg::Path { ident, ty } => {
			let key = ident.to_string();
			quote! {
				let #ident: #ty = match params.get::<#ty>(#key) {
					Some(Ok(v)) => v,
					Some(Err(_)) | None => {
						return Box::pin(async { wgui::wui::route_handler::RouteResult::NotFound });
					}
				};
			}
		}
		RouteArg::Form { ident, ty } => quote! {
			let #ident: #ty = match form.decode::<#ty>() {
				Ok(value) => value,
				Err(_) => return Box::pin(async { wgui::wui::route_handler::RouteResult::NotFound }),
			};
		},
	});

	let call_arg_idents = param_args.iter().map(|arg| {
		let ident = arg.ident();
		quote! { #ident }
	});
	let fn_call = if fn_is_async {
		quote! { #fn_ident(#ctx_ident, #(#call_arg_idents),*).await }
	} else {
		quote! { #fn_ident(#ctx_ident, #(#call_arg_idents),*) }
	};

	let expanded = quote! {
		#item_fn

		#template_impl

		#[allow(non_camel_case_types)]
		#[derive(Clone, Copy)]
		#[doc(hidden)]
		pub(crate) struct #marker_ident;

		impl wgui::wui::route_handler::RouteHandler for #marker_ident {
			type State = #state_type;

			fn path(&self) -> &str {
				#path_lit
			}

			fn method(&self) -> wgui::wui::route_handler::HttpMethod {
				#method_arm
			}

			#template_method

			fn call(
				self,
				ctx: ::std::sync::Arc<wgui::wui::runtime::Ctx<#state_type>>,
				params: wgui::wui::route_handler::PathParams,
				form: wgui::wui::route_handler::RouteFormData,
			) -> wgui::wui::route_handler::RouteFuture {
				#(#param_bindings)*
				Box::pin(async move {
					let #ctx_ident = &*ctx;
					let __result: #return_type = #fn_call;
					wgui::wui::route_handler::RouteResult::from(__result)
				})
			}
		}

		#[allow(non_upper_case_globals)]
		pub(crate) const #route_const_ident: #marker_ident = #marker_ident;
	};

	Ok(expanded)
}

/// Map a GET route to its conventional WUI template below `wui/pages`.
/// Static collection routes use `index`; an identifier terminal uses `show`;
/// and a static terminal after an identifier names the action page.
fn standard_route_template(path: &str) -> String {
	if path == "/*" {
		return "pages/not_found".to_string();
	}
	let segments = path
		.trim_matches('/')
		.split('/')
		.filter(|segment| !segment.is_empty())
		.collect::<Vec<_>>();
	if segments.is_empty() {
		return "pages/index".to_string();
	}
	let last_param = segments
		.last()
		.is_some_and(|segment| segment.starts_with(':'));
	let leaf = if last_param {
		"show"
	} else if segments.iter().any(|segment| segment.starts_with(':')) {
		segments.last().copied().unwrap_or("index")
	} else {
		"index"
	};
	let directories = if last_param || segments.iter().any(|segment| segment.starts_with(':')) {
		&segments[..segments.len().saturating_sub(1)]
	} else {
		&segments[..]
	};
	let mut output = String::from("pages");
	for segment in directories {
		if !segment.starts_with(':') && *segment != "*" {
			output.push('/');
			output.push_str(segment);
		}
	}
	output.push('/');
	output.push_str(leaf);
	output
}

type TokenStream2 = proc_macro2::TokenStream;

/// Extract the `ctx: &Ctx<T>` first arg, returning the user's ident for `ctx`
/// and the `T` (AppState type).
fn extract_ctx_arg(sig: &Signature) -> syn::Result<(syn::Ident, Type)> {
	let Some(FnArg::Typed(pat_ty)) = sig.inputs.first().cloned() else {
		return Err(syn::Error::new_spanned(
			&sig.ident,
			"#[route] handler must take `ctx: &Ctx<T>` as its first argument",
		));
	};
	let ctx_ident = match &*pat_ty.pat {
		Pat::Ident(pat_ident) => pat_ident.ident.clone(),
		other => {
			return Err(syn::Error::new_spanned(
				other,
				"#[route] first argument must be a plain ident bound to the Ctx",
			))
		}
	};
	let state_type = match &*pat_ty.ty {
		Type::Reference(r) => extract_ctx_generic(&r.elem)?,
		other => {
			return Err(syn::Error::new_spanned(
				other,
				"#[route] first argument must be `&Ctx<T>` — got a non-reference type",
			))
		}
	};
	Ok((ctx_ident, state_type))
}

/// Given a `Type::Path` for `Ctx<...>`, return the inner generic arg.
fn extract_ctx_generic(ty: &Type) -> syn::Result<Type> {
	let Type::Path(type_path) = ty else {
		return Err(syn::Error::new_spanned(
			ty,
			"#[route] ctx argument is not a path type; expected `Ctx<T>`",
		));
	};
	let last_seg = type_path.path.segments.last().ok_or_else(|| {
		syn::Error::new_spanned(ty, "#[route] ctx argument path is empty; expected `Ctx<T>`")
	})?;
	if last_seg.ident != "Ctx" {
		return Err(syn::Error::new_spanned(
			&last_seg.ident,
			"#[route] first argument must be `Ctx<T>`; got another type",
		));
	}
	let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments else {
		return Err(syn::Error::new_spanned(
			&last_seg.ident,
			"#[route] ctx must be parameterized: `Ctx<T>`",
		));
	};
	let first_arg = args.args.first().ok_or_else(|| {
		syn::Error::new_spanned(
			&last_seg.ident,
			"#[route] Ctx<T> missing its type parameter",
		)
	})?;
	let syn::GenericArgument::Type(t) = first_arg else {
		return Err(syn::Error::new_spanned(
			first_arg,
			"#[route] first generic argument of Ctx<T> must be a type",
		));
	};
	Ok(t.clone())
}

enum RouteArg {
	Path { ident: syn::Ident, ty: Type },
	Form { ident: syn::Ident, ty: Type },
}

impl RouteArg {
	fn ident(&self) -> &syn::Ident {
		match self {
			Self::Path { ident, .. } | Self::Form { ident, .. } => ident,
		}
	}
}

/// Extract every non-`ctx` argument. Arguments whose names match a `:name`
/// path segment are decoded from `PathParams`; one remaining argument on a
/// POST route is decoded from the URL-encoded request body.
fn extract_param_args(
	sig: &Signature,
	path_param_names: &[String],
	method: &RouteMethod,
) -> syn::Result<Vec<RouteArg>> {
	let mut out = Vec::new();
	let mut has_form = false;
	for arg in sig.inputs.iter().skip(1) {
		let FnArg::Typed(pat_ty) = arg else {
			return Err(syn::Error::new_spanned(
				arg,
				"#[route] non-ctx args must be typed params (e.g. `id: u32`)",
			));
		};
		let ident = match &*pat_ty.pat {
			Pat::Ident(pat_ident) => pat_ident.ident.clone(),
			other => {
				return Err(syn::Error::new_spanned(
					other,
					"#[route] handler args must be plain ident bindings",
				))
			}
		};
		let ty = (*pat_ty.ty).clone();
		if path_param_names
			.iter()
			.any(|name| name == &ident.to_string())
		{
			out.push(RouteArg::Path { ident, ty });
		} else {
			if !matches!(method, RouteMethod::Post) {
				return Err(syn::Error::new_spanned(
					ident,
					"only POST #[route] handlers may take a form argument",
				));
			}
			if has_form {
				return Err(syn::Error::new_spanned(
					ident,
					"a #[route] handler may take at most one form argument",
				));
			}
			has_form = true;
			out.push(RouteArg::Form { ident, ty });
		}
	}
	Ok(out)
}

fn path_param_names(path: &str) -> Vec<String> {
	path.split('/')
		.filter_map(|segment| segment.strip_prefix(':'))
		.filter(|name| !name.is_empty())
		.map(str::to_string)
		.collect()
}

/// `action_toggle` -> `__ActionToggleRoute`
fn marker_ident(fn_ident: &syn::Ident) -> syn::Ident {
	let pascal = to_pascal_case(&fn_ident.to_string());
	format_ident!("__{pascal}Route")
}

/// `action_toggle` -> `action_toggle_route`
fn route_const_ident(fn_ident: &syn::Ident, handle_kind: &str) -> syn::Ident {
	format_ident!("{}_{}", fn_ident, handle_kind)
}

fn to_pascal_case(input: &str) -> String {
	let mut out = String::new();
	let mut cap = true;
	for ch in input.chars() {
		if ch == '_' {
			cap = true;
			continue;
		}
		if cap {
			for upper in ch.to_uppercase() {
				out.push(upper);
			}
			cap = false;
		} else {
			out.push(ch);
		}
	}
	out
}
