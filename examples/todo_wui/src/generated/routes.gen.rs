pub struct RouteDef {
	pub module: &'static str,
	pub route: &'static str,
}

pub const ROUTES: &[RouteDef] = &[
	RouteDef {
		module: "todo",
		route: "/",
	},
	RouteDef {
		module: "todo",
		route: "/todo/{todoId}",
	},
	RouteDef {
		module: "todo",
		route: "/{*wildcard}",
	},
];
