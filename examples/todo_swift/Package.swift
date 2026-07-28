// swift-tools-version: 5.9

import PackageDescription

let package = Package(
	name: "TodoSwift",
	platforms: [
		.iOS(.v16),
		.macOS(.v13),
	],
	products: [
		.library(
			name: "TodoSwift",
			targets: ["TodoSwift"]
		),
	],
	targets: [
		.target(name: "TodoSwift"),
	]
)
