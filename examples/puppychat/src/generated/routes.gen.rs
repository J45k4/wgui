pub struct RouteDef {
	pub module: &'static str,
	pub route: &'static str,
}

pub const ROUTES: &[RouteDef] = &[
	RouteDef { module: "puppychat", route: "/" },
	RouteDef { module: "puppychat", route: "/{*wildcard}" },
];
