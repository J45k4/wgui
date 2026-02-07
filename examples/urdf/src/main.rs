use log::Level;
use roxmltree::Document;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use wgui::*;

const JOINT_SLIDER_BASE_ID: u32 = 10_000;
const URDF_TEXTAREA_ID: u32 = 20_001;
const APPLY_URDF_BUTTON_ID: u32 = 20_002;
const VALUE_SCALE: f32 = 1000.0;

#[derive(Clone)]
struct LinkVisual {
	origin_xyz: [f32; 3],
	origin_rpy: [f32; 3],
	color_rgb: [u8; 3],
	geometry: VisualGeometry,
}

#[derive(Clone)]
enum VisualGeometry {
	Mesh { src: String, scale: [f32; 3] },
	Box { size: [f32; 3] },
	Cylinder { radius: f32, length: f32 },
	Sphere { radius: f32 },
}

#[derive(Clone)]
struct LinkDef {
	name: String,
	visuals: Vec<LinkVisual>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JointType {
	Fixed,
	Revolute,
	Continuous,
	Prismatic,
	Other,
}

#[derive(Clone)]
struct JointDef {
	name: String,
	joint_type: JointType,
	parent: String,
	child: String,
	origin_xyz: [f32; 3],
	origin_rpy: [f32; 3],
	axis: [f32; 3],
	lower: Option<f32>,
	upper: Option<f32>,
}

#[derive(Clone)]
struct RobotModel {
	links: HashMap<String, LinkDef>,
	children_by_parent: HashMap<String, Vec<usize>>,
	joints: Vec<JointDef>,
	roots: Vec<String>,
	movable_joint_indices: Vec<usize>,
}

#[derive(Clone)]
struct State {
	robot: RobotModel,
	joint_values: Vec<i32>,
	urdf_label: String,
	urdf_xml: String,
	urdf_base_dir: PathBuf,
	status_message: String,
}

fn parse_vec3(input: Option<&str>) -> [f32; 3] {
	let mut out = [0.0f32, 0.0, 0.0];
	let Some(raw) = input else {
		return out;
	};
	for (index, part) in raw.split_whitespace().take(3).enumerate() {
		if let Ok(value) = part.parse::<f32>() {
			out[index] = value;
		}
	}
	out
}

fn parse_vec3_or(input: Option<&str>, default: [f32; 3]) -> [f32; 3] {
	match input {
		Some(raw) => {
			let parsed = parse_vec3(Some(raw));
			if parsed == [0.0, 0.0, 0.0] && default != [0.0, 0.0, 0.0] {
				default
			} else {
				parsed
			}
		}
		None => default,
	}
}

fn parse_f32_or(input: Option<&str>, default: f32) -> f32 {
	match input.and_then(|s| s.parse::<f32>().ok()) {
		Some(value) => value,
		None => default,
	}
}

fn parse_color_rgb(visual_node: roxmltree::Node<'_, '_>) -> [u8; 3] {
	let rgba = visual_node
		.children()
		.find(|n| n.has_tag_name("material"))
		.and_then(|mat| mat.children().find(|n| n.has_tag_name("color")))
		.and_then(|color| color.attribute("rgba"));

	let Some(rgba) = rgba else {
		return [190, 190, 205];
	};
	let parts: Vec<f32> = rgba
		.split_whitespace()
		.filter_map(|part| part.parse::<f32>().ok())
		.collect();
	if parts.len() < 3 {
		return [190, 190, 205];
	}
	let to_u8 = |v: f32| -> u8 { (v.clamp(0.0, 1.0) * 255.0).round() as u8 };
	[to_u8(parts[0]), to_u8(parts[1]), to_u8(parts[2])]
}

fn parse_joint_type(input: &str) -> JointType {
	match input {
		"fixed" => JointType::Fixed,
		"revolute" => JointType::Revolute,
		"continuous" => JointType::Continuous,
		"prismatic" => JointType::Prismatic,
		_ => JointType::Other,
	}
}

fn path_to_assets_url(path: &Path) -> Option<String> {
	let mut saw_assets = false;
	let mut rest = Vec::<String>::new();
	for component in path.components() {
		match component {
			Component::Normal(part) => {
				let text = part.to_string_lossy().to_string();
				if !saw_assets {
					if text == "assets" {
						saw_assets = true;
					}
					continue;
				}
				rest.push(text);
			}
			Component::CurDir => {}
			Component::ParentDir => return None,
			Component::RootDir | Component::Prefix(_) => {}
		}
	}
	if !saw_assets || rest.is_empty() {
		None
	} else {
		Some(format!("/assets/{}", rest.join("/")))
	}
}

fn path_to_workspace_url(path: &Path, workspace_root: &Path) -> Option<String> {
	let relative = path.strip_prefix(workspace_root).ok()?;
	let mut rest = Vec::<String>::new();
	for component in relative.components() {
		match component {
			Component::Normal(part) => rest.push(part.to_string_lossy().to_string()),
			Component::CurDir => {}
			Component::ParentDir => return None,
			Component::RootDir | Component::Prefix(_) => return None,
		}
	}
	if rest.is_empty() {
		None
	} else {
		Some(format!("/fs/{}", rest.join("/")))
	}
}

fn parse_mesh_src(urdf_dir: &Path, workspace_root: &Path, filename: &str) -> Option<String> {
	let mut package_path: Option<PathBuf> = None;
	let trimmed = if let Some(without_scheme) = filename.strip_prefix("package://") {
		let mut parts = without_scheme.splitn(2, '/');
		let _package_name = parts.next();
		if let Some(rest) = parts.next() {
			package_path = Some(PathBuf::from(rest));
			rest
		} else {
			without_scheme
		}
	} else {
		filename
	};
	let mesh_path = Path::new(trimmed);
	let mut candidates = Vec::<PathBuf>::new();

	if mesh_path.is_absolute() {
		candidates.push(mesh_path.to_path_buf());
	} else {
		candidates.push(urdf_dir.join(mesh_path));
		if let Some(file_name) = mesh_path.file_name() {
			candidates.push(urdf_dir.join(file_name));
			candidates.push(urdf_dir.join("meshes").join(file_name));
		}
		if let Some(parent) = urdf_dir.parent() {
			candidates.push(parent.join(mesh_path));
		}
	}

	if let Some(package_rel) = package_path {
		candidates.push(urdf_dir.join(&package_rel));
		if let Some(parent) = urdf_dir.parent() {
			candidates.push(parent.join(&package_rel));
		}
		if let Some(file_name) = package_rel.file_name() {
			candidates.push(urdf_dir.join(file_name));
		}
	}

	for candidate in candidates {
		if let Ok(canonical) = std::fs::canonicalize(&candidate) {
			if let Some(url) = path_to_assets_url(&canonical) {
				return Some(url);
			}
			if let Some(url) = path_to_workspace_url(&canonical, workspace_root) {
				return Some(url);
			}
		} else if candidate.exists() {
			if let Some(url) = path_to_assets_url(&candidate) {
				return Some(url);
			}
			if let Some(url) = path_to_workspace_url(&candidate, workspace_root) {
				return Some(url);
			}
		}
	}

	None
}

fn parse_robot_from_xml(
	xml: &str,
	urdf_dir: &Path,
	workspace_root: &Path,
) -> Result<RobotModel, String> {
	let doc = Document::parse(&xml).map_err(|err| format!("failed to parse URDF XML: {err}"))?;

	let mut links: HashMap<String, LinkDef> = HashMap::new();
	for link_node in doc.descendants().filter(|n| n.has_tag_name("link")) {
		let Some(name) = link_node.attribute("name") else {
			continue;
		};
		let mut visuals = Vec::new();
		for visual_node in link_node.children().filter(|n| n.has_tag_name("visual")) {
			let origin_node = visual_node.children().find(|n| n.has_tag_name("origin"));
			let origin_xyz = parse_vec3(origin_node.and_then(|n| n.attribute("xyz")));
			let origin_rpy = parse_vec3(origin_node.and_then(|n| n.attribute("rpy")));
			let color_rgb = parse_color_rgb(visual_node);

			let geometry_node = visual_node.children().find(|n| n.has_tag_name("geometry"));
			let Some(geometry_node) = geometry_node else {
				continue;
			};

			let geometry = if let Some(mesh_node) =
				geometry_node.children().find(|n| n.has_tag_name("mesh"))
			{
				let Some(mesh_filename) = mesh_node.attribute("filename") else {
					continue;
				};
				let mesh_scale = parse_vec3_or(mesh_node.attribute("scale"), [1.0, 1.0, 1.0]);
				let Some(mesh_src) = parse_mesh_src(urdf_dir, workspace_root, mesh_filename) else {
					continue;
				};
				VisualGeometry::Mesh {
					src: mesh_src,
					scale: mesh_scale,
				}
			} else if let Some(box_node) = geometry_node.children().find(|n| n.has_tag_name("box"))
			{
				VisualGeometry::Box {
					size: parse_vec3_or(box_node.attribute("size"), [0.05, 0.05, 0.05]),
				}
			} else if let Some(cylinder_node) = geometry_node
				.children()
				.find(|n| n.has_tag_name("cylinder"))
			{
				VisualGeometry::Cylinder {
					radius: parse_f32_or(cylinder_node.attribute("radius"), 0.05),
					length: parse_f32_or(cylinder_node.attribute("length"), 0.1),
				}
			} else if let Some(sphere_node) =
				geometry_node.children().find(|n| n.has_tag_name("sphere"))
			{
				VisualGeometry::Sphere {
					radius: parse_f32_or(sphere_node.attribute("radius"), 0.05),
				}
			} else {
				continue;
			};

			visuals.push(LinkVisual {
				origin_xyz,
				origin_rpy,
				color_rgb,
				geometry,
			});
		}

		links.insert(
			name.to_string(),
			LinkDef {
				name: name.to_string(),
				visuals,
			},
		);
	}

	let mut joints = Vec::new();
	for joint_node in doc.descendants().filter(|n| n.has_tag_name("joint")) {
		let Some(name) = joint_node.attribute("name") else {
			continue;
		};
		let joint_type = parse_joint_type(joint_node.attribute("type").unwrap_or("fixed"));
		let parent = joint_node
			.children()
			.find(|n| n.has_tag_name("parent"))
			.and_then(|n| n.attribute("link"))
			.unwrap_or("world")
			.to_string();
		let Some(child) = joint_node
			.children()
			.find(|n| n.has_tag_name("child"))
			.and_then(|n| n.attribute("link"))
			.map(|s| s.to_string())
		else {
			continue;
		};
		let origin_node = joint_node.children().find(|n| n.has_tag_name("origin"));
		let origin_xyz = parse_vec3(origin_node.and_then(|n| n.attribute("xyz")));
		let origin_rpy = parse_vec3(origin_node.and_then(|n| n.attribute("rpy")));
		let axis = parse_vec3(
			joint_node
				.children()
				.find(|n| n.has_tag_name("axis"))
				.and_then(|n| n.attribute("xyz")),
		);

		let limit_node = joint_node.children().find(|n| n.has_tag_name("limit"));
		let lower = limit_node
			.and_then(|n| n.attribute("lower"))
			.and_then(|s| s.parse::<f32>().ok());
		let upper = limit_node
			.and_then(|n| n.attribute("upper"))
			.and_then(|s| s.parse::<f32>().ok());

		joints.push(JointDef {
			name: name.to_string(),
			joint_type,
			parent,
			child,
			origin_xyz,
			origin_rpy,
			axis,
			lower,
			upper,
		});
	}

	let mut children_by_parent: HashMap<String, Vec<usize>> = HashMap::new();
	let mut has_parent: HashSet<String> = HashSet::new();
	for (joint_index, joint) in joints.iter().enumerate() {
		children_by_parent
			.entry(joint.parent.clone())
			.or_default()
			.push(joint_index);
		has_parent.insert(joint.child.clone());
	}

	let mut roots: Vec<String> = links
		.keys()
		.filter(|name| !has_parent.contains(*name))
		.cloned()
		.collect();
	roots.sort();

	if let Some(world_children) = children_by_parent.get("world") {
		for idx in world_children {
			if !roots.iter().any(|name| name == &joints[*idx].child) {
				roots.push(joints[*idx].child.clone());
			}
		}
	}

	let movable_joint_indices = joints
		.iter()
		.enumerate()
		.filter_map(|(index, joint)| {
			(joint.joint_type == JointType::Revolute
				|| joint.joint_type == JointType::Continuous
				|| joint.joint_type == JointType::Prismatic)
				.then_some(index)
		})
		.collect();

	Ok(RobotModel {
		links,
		children_by_parent,
		joints,
		roots,
		movable_joint_indices,
	})
}

fn parse_robot(urdf_path: &Path, workspace_root: &Path) -> Result<(RobotModel, String), String> {
	let xml = std::fs::read_to_string(urdf_path)
		.map_err(|err| format!("failed to read URDF {}: {err}", urdf_path.display()))?;
	let urdf_dir = urdf_path.parent().unwrap_or(Path::new("."));
	let robot = parse_robot_from_xml(&xml, urdf_dir, workspace_root)?;
	Ok((robot, xml))
}

fn axis_to_euler(axis: [f32; 3], value: f32) -> [f32; 3] {
	let eps = 0.2;
	if axis[0].abs() > 1.0 - eps && axis[1].abs() < eps && axis[2].abs() < eps {
		return [value * axis[0].signum(), 0.0, 0.0];
	}
	if axis[1].abs() > 1.0 - eps && axis[0].abs() < eps && axis[2].abs() < eps {
		return [0.0, value * axis[1].signum(), 0.0];
	}
	if axis[2].abs() > 1.0 - eps && axis[0].abs() < eps && axis[1].abs() < eps {
		return [0.0, 0.0, value * axis[2].signum()];
	}
	[0.0, 0.0, value]
}

fn joint_slider_range(joint: &JointDef) -> (i32, i32) {
	match joint.joint_type {
		JointType::Revolute => {
			let min = joint.lower.unwrap_or(-std::f32::consts::PI);
			let max = joint.upper.unwrap_or(std::f32::consts::PI);
			((min * VALUE_SCALE) as i32, (max * VALUE_SCALE) as i32)
		}
		JointType::Continuous => (
			(-std::f32::consts::PI * VALUE_SCALE) as i32,
			(std::f32::consts::PI * VALUE_SCALE) as i32,
		),
		JointType::Prismatic => {
			let min = joint.lower.unwrap_or(-0.05);
			let max = joint.upper.unwrap_or(0.05);
			((min * VALUE_SCALE) as i32, (max * VALUE_SCALE) as i32)
		}
		_ => (0, 0),
	}
}

fn value_to_joint_units(joint: &JointDef, slider_value: i32) -> f32 {
	match joint.joint_type {
		JointType::Revolute | JointType::Continuous | JointType::Prismatic => {
			slider_value as f32 / VALUE_SCALE
		}
		_ => 0.0,
	}
}

fn push_link_visuals(link: &LinkDef, id_gen: &mut u32, children: &mut Vec<ThreeNode>) {
	for visual in &link.visuals {
		*id_gen += 1;
		let mesh_id = *id_gen;
		*id_gen += 1;
		let geom_id = *id_gen;
		*id_gen += 1;
		let mat_id = *id_gen;
		*id_gen += 1;
		let visual_group_id = *id_gen;
		let geometry_node = match &visual.geometry {
			VisualGeometry::Mesh { src, .. } => {
				stl_geometry(geom_id).prop("src", ThreePropValue::String { value: src.clone() })
			}
			VisualGeometry::Box { size } => box_geometry(geom_id)
				.prop("width", ThreePropValue::Number { value: size[0] })
				.prop("height", ThreePropValue::Number { value: size[1] })
				.prop("depth", ThreePropValue::Number { value: size[2] }),
			VisualGeometry::Cylinder { radius, length } => cylinder_geometry(geom_id)
				.prop("radiusTop", ThreePropValue::Number { value: *radius })
				.prop("radiusBottom", ThreePropValue::Number { value: *radius })
				.prop("height", ThreePropValue::Number { value: *length }),
			VisualGeometry::Sphere { radius } => {
				sphere_geometry(geom_id).prop("radius", ThreePropValue::Number { value: *radius })
			}
		};

		let mut mesh_node = mesh(
			mesh_id,
			[
				geometry_node,
				mesh_standard_material(mat_id)
					.prop(
						"color",
						ThreePropValue::Color {
							r: visual.color_rgb[0],
							g: visual.color_rgb[1],
							b: visual.color_rgb[2],
							a: None,
						},
					)
					.prop("metalness", ThreePropValue::Number { value: 0.15 })
					.prop("roughness", ThreePropValue::Number { value: 0.7 }),
			],
		);
		if let VisualGeometry::Cylinder { .. } = &visual.geometry {
			mesh_node = mesh_node.prop(
				"rotation",
				ThreePropValue::Vec3 {
					x: std::f32::consts::FRAC_PI_2,
					y: 0.0,
					z: 0.0,
				},
			);
		}
		if let VisualGeometry::Mesh { scale, .. } = &visual.geometry {
			mesh_node = mesh_node.prop(
				"scale",
				ThreePropValue::Vec3 {
					x: scale[0],
					y: scale[1],
					z: scale[2],
				},
			);
		}

		children.push(
			group(visual_group_id, [mesh_node])
				.prop(
					"position",
					ThreePropValue::Vec3 {
						x: visual.origin_xyz[0],
						y: visual.origin_xyz[1],
						z: visual.origin_xyz[2],
					},
				)
				.prop(
					"rotation",
					ThreePropValue::Vec3 {
						x: visual.origin_rpy[0],
						y: visual.origin_rpy[1],
						z: visual.origin_rpy[2],
					},
				),
		);
	}
}

fn build_link_subtree(
	robot: &RobotModel,
	link_name: &str,
	joint_values: &[i32],
	id_gen: &mut u32,
) -> Option<ThreeNode> {
	let link = robot.links.get(link_name)?;
	*id_gen += 1;
	let link_group_id = *id_gen;

	let mut children = Vec::new();
	push_link_visuals(link, id_gen, &mut children);

	if let Some(child_joint_indices) = robot.children_by_parent.get(link_name) {
		for joint_index in child_joint_indices {
			let joint = &robot.joints[*joint_index];
			*id_gen += 1;
			let origin_group_id = *id_gen;
			*id_gen += 1;
			let motion_group_id = *id_gen;

			let mut motion_group = group(motion_group_id, []);
			let joint_value = joint_values.get(*joint_index).copied().unwrap_or(0);
			let unit_value = value_to_joint_units(joint, joint_value);
			match joint.joint_type {
				JointType::Revolute | JointType::Continuous => {
					let delta = axis_to_euler(joint.axis, unit_value);
					motion_group = motion_group.prop(
						"rotation",
						ThreePropValue::Vec3 {
							x: delta[0],
							y: delta[1],
							z: delta[2],
						},
					);
				}
				JointType::Prismatic => {
					motion_group = motion_group.prop(
						"position",
						ThreePropValue::Vec3 {
							x: joint.axis[0] * unit_value,
							y: joint.axis[1] * unit_value,
							z: joint.axis[2] * unit_value,
						},
					);
				}
				_ => {}
			}

			if let Some(child_tree) = build_link_subtree(robot, &joint.child, joint_values, id_gen)
			{
				motion_group = motion_group.child(child_tree);
			}

			children.push(
				group(origin_group_id, [motion_group])
					.prop(
						"position",
						ThreePropValue::Vec3 {
							x: joint.origin_xyz[0],
							y: joint.origin_xyz[1],
							z: joint.origin_xyz[2],
						},
					)
					.prop(
						"rotation",
						ThreePropValue::Vec3 {
							x: joint.origin_rpy[0],
							y: joint.origin_rpy[1],
							z: joint.origin_rpy[2],
						},
					),
			);
		}
	}

	Some(group(link_group_id, children).prop(
		"name",
		ThreePropValue::String {
			value: link.name.clone(),
		},
	))
}

fn render(state: &State) -> Item {
	let mut id_gen = 100;
	let mut scene_children = vec![
		perspective_camera(2)
			.prop(
				"position",
				ThreePropValue::Vec3 {
					x: 0.28,
					y: 0.2,
					z: 0.35,
				},
			)
			.prop(
				"lookAt",
				ThreePropValue::Vec3 {
					x: -0.05,
					y: 0.0,
					z: -0.05,
				},
			)
			.prop("active", ThreePropValue::Bool { value: true }),
		ambient_light(3).prop("intensity", ThreePropValue::Number { value: 0.65 }),
		directional_light(4)
			.prop(
				"position",
				ThreePropValue::Vec3 {
					x: 0.5,
					y: 1.0,
					z: 0.4,
				},
			)
			.prop("intensity", ThreePropValue::Number { value: 1.1 }),
	];

	let mut model_children: Vec<ThreeNode> = Vec::new();
	for root in &state.robot.roots {
		if let Some(root_tree) =
			build_link_subtree(&state.robot, root, &state.joint_values, &mut id_gen)
		{
			model_children.push(root_tree);
		}
	}
	scene_children.push(group(90, model_children));

	let three_panel = three_view(scene(1, scene_children))
		.height(680)
		.border("1px solid #303030")
		.grow(1);

	let mut controls = vec![
		text("URDF viewer").margin_bottom(8),
		text(&format!("Model: {}", state.urdf_label)).margin_bottom(8),
		text(&state.status_message).margin_bottom(8),
		text(&format!(
			"Loaded links: {}, joints: {}",
			state.robot.links.len(),
			state.robot.joints.len()
		))
		.margin_bottom(12),
	];

	if state.robot.movable_joint_indices.is_empty() {
		controls
			.push(text("No controllable joints found (URDF joints are fixed).").margin_bottom(8));
	}

	for (slider_slot, joint_index) in state.robot.movable_joint_indices.iter().enumerate() {
		let joint = &state.robot.joints[*joint_index];
		let slider_id = JOINT_SLIDER_BASE_ID + slider_slot as u32;
		let value = state.joint_values.get(*joint_index).copied().unwrap_or(0);
		let (min, max) = joint_slider_range(joint);
		controls.push(text(&joint.name));
		controls.push(
			slider()
				.id(slider_id)
				.min(min)
				.max(max)
				.ivalue(value)
				.step(1),
		);
		controls
			.push(text(&format!("{:.4}", value_to_joint_units(joint, value))).margin_bottom(10));
	}

	let controls_panel = vstack(controls)
		.width(320)
		.padding(12)
		.border("1px solid #d0d0d0")
		.background_color("#fafafa")
		.overflow("scroll")
		.height(680);
	let editor_panel = vstack([
		text("URDF XML"),
		textarea()
			.id(URDF_TEXTAREA_ID)
			.svalue(&state.urdf_xml)
			.placeholder("Paste URDF XML here")
			.height(260),
		button("Apply URDF from text")
			.id(APPLY_URDF_BUTTON_ID)
			.margin_top(8),
	])
	.padding(12)
	.border("1px solid #d0d0d0")
	.background_color("#fafafa");

	vstack([
		hstack([three_panel, controls_panel]).spacing(14),
		editor_panel,
	])
	.spacing(12)
	.padding(14)
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let urdf_arg = match std::env::args().nth(1) {
		Some(arg) => arg,
		None => {
			eprintln!("missing required argument: <path-to-urdf>");
			eprintln!("usage: cargo run -p urdf -- <path-to-urdf>");
			std::process::exit(2);
		}
	};
	let urdf_path = PathBuf::from(&urdf_arg);
	let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
	let (robot, urdf_xml) =
		parse_robot(&urdf_path, &workspace_root).unwrap_or_else(|err| panic!("{err}"));
	let urdf_base_dir = urdf_path.parent().unwrap_or(Path::new(".")).to_path_buf();
	let mut state = State {
		joint_values: vec![0; robot.joints.len()],
		robot,
		urdf_label: urdf_arg,
		urdf_xml,
		urdf_base_dir,
		status_message: "Loaded URDF from path".to_string(),
	};

	let mut client_ids = HashSet::new();
	let mut wgui = Wgui::new("0.0.0.0:12348".parse().unwrap());

	while let Some(message) = wgui.next().await {
		let client_id = message.client_id;
		match message.event {
			ClientEvent::Disconnected { id: _ } => {
				client_ids.remove(&client_id);
			}
			ClientEvent::Connected { id: _ } => {
				wgui.render(client_id, render(&state)).await;
				client_ids.insert(client_id);
			}
			ClientEvent::OnSliderChange(change) => {
				if change.id >= JOINT_SLIDER_BASE_ID {
					let slot = (change.id - JOINT_SLIDER_BASE_ID) as usize;
					if let Some(joint_index) = state.robot.movable_joint_indices.get(slot) {
						if let Some(value) = state.joint_values.get_mut(*joint_index) {
							*value = change.value;
						}
					}
				}
			}
			ClientEvent::OnTextChanged(change) => {
				if change.id == URDF_TEXTAREA_ID {
					state.urdf_xml = change.value;
				}
			}
			ClientEvent::OnClick(click) => {
				if click.id == APPLY_URDF_BUTTON_ID {
					match parse_robot_from_xml(
						&state.urdf_xml,
						&state.urdf_base_dir,
						&workspace_root,
					) {
						Ok(robot) => {
							let old_values = state.joint_values.clone();
							state.joint_values = vec![0; robot.joints.len()];
							for (slot, value) in old_values.iter().enumerate() {
								if slot < state.joint_values.len() {
									state.joint_values[slot] = *value;
								}
							}
							state.robot = robot;
							state.status_message = "Applied URDF from editor".to_string();
						}
						Err(err) => {
							state.status_message = err;
						}
					}
				}
			}
			_ => {}
		}

		for id in &client_ids {
			wgui.render(*id, render(&state)).await;
		}
	}
}
