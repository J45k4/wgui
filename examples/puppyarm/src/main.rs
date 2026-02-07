use log::Level;
use roxmltree::Document;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use wgui::*;

const JOINT_SLIDER_BASE_ID: u32 = 10_000;
const VALUE_SCALE: f32 = 1000.0;

#[derive(Clone)]
struct LinkVisual {
	mesh_src: String,
	origin_xyz: [f32; 3],
	origin_rpy: [f32; 3],
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

fn parse_joint_type(input: &str) -> JointType {
	match input {
		"fixed" => JointType::Fixed,
		"revolute" => JointType::Revolute,
		"continuous" => JointType::Continuous,
		"prismatic" => JointType::Prismatic,
		_ => JointType::Other,
	}
}

fn parse_mesh_src(filename: &str) -> Option<String> {
	let file_name = Path::new(filename).file_name()?.to_str()?;
	Some(format!("/assets/puppyarm/meshes/{file_name}"))
}

fn parse_robot(urdf_path: &str) -> RobotModel {
	let xml = std::fs::read_to_string(urdf_path)
		.unwrap_or_else(|err| panic!("failed to read URDF {urdf_path}: {err}"));
	let doc = Document::parse(&xml)
		.unwrap_or_else(|err| panic!("failed to parse URDF {urdf_path}: {err}"));

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

			let mesh_filename = visual_node
				.children()
				.find(|n| n.has_tag_name("geometry"))
				.and_then(|geometry| geometry.children().find(|n| n.has_tag_name("mesh")))
				.and_then(|mesh| mesh.attribute("filename"));

			let Some(mesh_filename) = mesh_filename else {
				continue;
			};
			let Some(mesh_src) = parse_mesh_src(mesh_filename) else {
				continue;
			};

			visuals.push(LinkVisual {
				mesh_src,
				origin_xyz,
				origin_rpy,
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

	RobotModel {
		links,
		children_by_parent,
		joints,
		roots,
		movable_joint_indices,
	}
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

		children.push(
			mesh(
				mesh_id,
				[
					stl_geometry(geom_id).prop(
						"src",
						ThreePropValue::String {
							value: visual.mesh_src.clone(),
						},
					),
					mesh_standard_material(mat_id)
						.prop(
							"color",
							ThreePropValue::Color {
								r: 190,
								g: 190,
								b: 205,
								a: None,
							},
						)
						.prop("metalness", ThreePropValue::Number { value: 0.15 })
						.prop("roughness", ThreePropValue::Number { value: 0.7 }),
				],
			)
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

	for root in &state.robot.roots {
		if let Some(root_tree) =
			build_link_subtree(&state.robot, root, &state.joint_values, &mut id_gen)
		{
			scene_children.push(root_tree);
		}
	}

	let three_panel = three_view(scene(1, scene_children))
		.height(680)
		.border("1px solid #303030")
		.grow(1);

	let mut controls = vec![
		text("PuppyArm URDF viewer").margin_bottom(8),
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

	hstack([three_panel, controls_panel])
		.spacing(14)
		.padding(14)
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let robot = parse_robot("assets/puppyarm/puppyarm.urdf");
	let mut state = State {
		joint_values: vec![0; robot.joints.len()],
		robot,
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
			_ => {}
		}

		for id in &client_ids {
			wgui.render(*id, render(&state)).await;
		}
	}
}
