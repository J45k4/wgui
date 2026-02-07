use log::Level;
use std::collections::HashSet;
use wgui::*;

const ROTATION_SLIDER_ID: u32 = 1;

struct State {
	rotation_ticks: i32,
}

fn render(state: &State) -> Item {
	let rotation = state.rotation_ticks as f32 / 100.0;
	let scene = scene(
		1,
		[
			perspective_camera(2)
				.prop(
					"position",
					ThreePropValue::Vec3 {
						x: 2.5,
						y: 2.0,
						z: 4.0,
					},
				)
				.prop("active", ThreePropValue::Bool { value: true }),
			ambient_light(3).prop("intensity", ThreePropValue::Number { value: 0.4 }),
			directional_light(4)
				.prop(
					"position",
					ThreePropValue::Vec3 {
						x: 2.0,
						y: 4.0,
						z: 3.0,
					},
				)
				.prop("intensity", ThreePropValue::Number { value: 1.0 }),
			mesh(
				5,
				[
					box_geometry(6),
					mesh_standard_material(7)
						.prop(
							"color",
							ThreePropValue::Color {
								r: 80,
								g: 140,
								b: 220,
								a: None,
							},
						)
						.prop("metalness", ThreePropValue::Number { value: 0.2 })
						.prop("roughness", ThreePropValue::Number { value: 0.6 }),
				],
			)
			.prop(
				"rotation",
				ThreePropValue::Vec3 {
					x: 0.0,
					y: rotation,
					z: 0.0,
				},
			)
			.prop(
				"position",
				ThreePropValue::Vec3 {
					x: -1.5,
					y: 0.0,
					z: 0.0,
				},
			),
			mesh(
				8,
				[
					stl_geometry(9).prop(
						"src",
						ThreePropValue::String {
							value: "/assets/puppyarm/meshes/Part_1__2.stl".to_string(),
						},
					),
					mesh_standard_material(10)
						.prop(
							"color",
							ThreePropValue::Color {
								r: 220,
								g: 130,
								b: 80,
								a: None,
							},
						)
						.prop("metalness", ThreePropValue::Number { value: 0.1 })
						.prop("roughness", ThreePropValue::Number { value: 0.8 }),
				],
			)
			.prop(
				"position",
				ThreePropValue::Vec3 {
					x: 1.5,
					y: -0.4,
					z: 0.0,
				},
			)
			.prop(
				"rotation",
				ThreePropValue::Vec3 {
					x: -1.57,
					y: rotation * 0.5,
					z: 0.0,
				},
			)
			.prop(
				"scale",
				ThreePropValue::Vec3 {
					x: 0.03,
					y: 0.03,
					z: 0.03,
				},
			),
		],
	);

	vstack([
		text("3D demo (cube + STL)").margin_bottom(10),
		hstack([
			three_view(scene)
				.height(420)
				.grow(1)
				.border("1px solid #333"),
			vstack([
				text("Rotate cube"),
				slider()
					.id(ROTATION_SLIDER_ID)
					.min(0)
					.max(628)
					.ivalue(state.rotation_ticks)
					.step(1),
				text(&format!("{:.2} rad", rotation)),
			])
			.width(220)
			.padding(10)
			.border("1px solid #ddd")
			.background_color("#fafafa"),
		])
		.spacing(16),
	])
	.padding(16)
	.spacing(12)
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let mut state = State { rotation_ticks: 0 };
	let mut client_ids = HashSet::new();
	let mut wgui = Wgui::new("0.0.0.0:12347".parse().unwrap());

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
				if change.id == ROTATION_SLIDER_ID {
					state.rotation_ticks = change.value;
				}
			}
			_ => {}
		}

		for id in &client_ids {
			wgui.render(*id, render(&state)).await;
		}
	}
}
