use crate::edit_distance::get_minimum_edits;
use crate::edit_distance::EditOperation;
use crate::gui::Item;
use crate::gui::ItemPayload;
use crate::types::AddBack;
use crate::types::AddFront;
use crate::types::ClientAction;
use crate::types::InsertAt;
use crate::types::ItemPath;
use crate::types::PropKey;
use crate::types::RemoveInx;
use crate::types::Replace;
use crate::types::SetProp;
use crate::types::Value;

fn inner_diff(changes: &mut Vec<ClientAction>, old: &Item, new: &Item, path: ItemPath) {
	log::trace!("{:?} inner_dif", path);
	let mut sets: Vec<SetProp> = Vec::new();

	match (&old.payload, &new.payload) {
		(ItemPayload::Layout(old_layout), ItemPayload::Layout(new_layout)) => {
			log::trace!("{:?} layout", path);

			if old_layout.flex != new_layout.flex {
				let flex = match new_layout.flex {
					crate::gui::FlexDirection::Row => "row",
					crate::gui::FlexDirection::Column => "column",
				};
				sets.push(SetProp {
					key: PropKey::FlexDirection,
					value: Value::String(flex.to_string()),
				});
			}

			if old_layout.wrap != new_layout.wrap
				|| old_layout.horizontal_resize != new_layout.horizontal_resize
				|| old_layout.vresize != new_layout.vresize
				|| old_layout.hresize != new_layout.hresize
				|| old_layout.pos != new_layout.pos
			{
				changes.push(ClientAction::Replace(Replace {
					path: path.clone(),
					item: new.clone(),
				}));
				return;
			}

			if old_layout.spacing != new_layout.spacing {
				println!("{:?} spacing is different", path);
				sets.push(SetProp {
					key: PropKey::Spacing,
					value: Value::Number(new_layout.spacing),
				})
			}

			let edits = get_minimum_edits(&old_layout.body, &new_layout.body);
			for edit in edits {
				match edit {
					EditOperation::InsertFirst(item) => {
						log::trace!("{:?} insert first", path);

						changes.push(ClientAction::AddFront(AddFront {
							path: path.clone(),
							item,
						}));
					}
					EditOperation::InsertAfter(index, item) => {
						log::trace!("{:?} insert after {}", path, index);

						changes.push(ClientAction::InsertAt(InsertAt {
							path: path.clone(),
							inx: index,
							item,
						}));
					}
					EditOperation::RemoveAt(index) => {
						log::trace!("{:?} remove at index {}", path, index);

						changes.push(ClientAction::RemoveInx(RemoveInx {
							path: path.clone(),
							inx: index,
						}));
					}
					EditOperation::ReplaceAt(i, item) => {
						log::trace!("{:?} replace at {}", path, i);

						let mut path = path.clone();
						path.push(i);

						log::trace!("{:?} new path: {:?}", path, path);

						inner_diff(changes, &old_layout.body[i], &item, path);
					}
					EditOperation::InsertBack(item) => {
						log::trace!("{:?} insert back", path);

						changes.push(ClientAction::AddBack(AddBack {
							path: path.clone(),
							item,
						}));
					}
				}
			}
		}
		(
			ItemPayload::ConnectionStatus {
				connected: old_connected,
				spacing: old_spacing,
				wrap: old_wrap,
				body: old_body,
				..
			},
			ItemPayload::ConnectionStatus {
				connected: new_connected,
				spacing: new_spacing,
				wrap: new_wrap,
				body: new_body,
				..
			},
		) => {
			if old_connected != new_connected || old_spacing != new_spacing || old_wrap != new_wrap
			{
				changes.push(ClientAction::Replace(Replace {
					path: path.clone(),
					item: new.clone(),
				}));
				return;
			}
			diff_children(changes, old_body, new_body, path.clone());
		}
		_ => {
			if old != new {
				log::trace!("{:?} old and new are different", path);

				changes.push(ClientAction::Replace(Replace {
					path: path.clone(),
					item: new.clone(),
				}));
			}
		}
	}

	if old.id != new.id {
		sets.push(SetProp {
			key: PropKey::ID,
			value: Value::Number(new.id),
		})
	}
	if old.grow != new.grow {
		sets.push(SetProp {
			key: PropKey::Grow,
			value: Value::Number(new.grow),
		});
	}
	if old.fill != new.fill {
		sets.push(SetProp {
			key: PropKey::Fill,
			value: Value::Number(new.fill as u32),
		});
	}
	if old.width != new.width {
		sets.push(SetProp {
			key: PropKey::Width,
			value: Value::Number(new.width),
		});
	}
	if old.height != new.height {
		sets.push(SetProp {
			key: PropKey::Height,
			value: Value::Number(new.height),
		});
	}
	if old.min_width != new.min_width {
		sets.push(SetProp {
			key: PropKey::MinWidth,
			value: Value::Number(new.min_width),
		});
	}
	if old.max_width != new.max_width {
		sets.push(SetProp {
			key: PropKey::MaxWidth,
			value: Value::Number(new.max_width),
		});
	}
	if old.min_height != new.min_height {
		sets.push(SetProp {
			key: PropKey::MinHeight,
			value: Value::Number(new.min_height),
		});
	}
	if old.max_height != new.max_height {
		sets.push(SetProp {
			key: PropKey::MaxHeight,
			value: Value::Number(new.max_height),
		});
	}
	if old.padding != new.padding {
		sets.push(SetProp {
			key: PropKey::Padding,
			value: Value::Number(new.padding as u32),
		});
	}

	if old.border != new.border {
		println!("{:?} border is different", path);
		sets.push(SetProp {
			key: PropKey::Border,
			value: Value::String(new.border.clone()),
		});
	}
	if old.background_color != new.background_color {
		// changes.push(ClientAction::SetStyle {
		// 	path: path.clone(),
		// 	prop: "background-color".to_string(),
		// 	value: new.background_color.to_string()
		// })
		sets.push(SetProp {
			key: PropKey::BackgroundColor,
			value: Value::String(new.background_color.clone()),
		});
	}
	if old.color != new.color {
		sets.push(SetProp {
			key: PropKey::Color,
			value: Value::String(new.color.clone()),
		});
	}
	if old.break_words != new.break_words {
		sets.push(SetProp {
			key: PropKey::BreakWords,
			value: Value::Number(new.break_words as u32),
		});
	}
	if old.overflow != new.overflow {
		sets.push(SetProp {
			key: PropKey::Overflow,
			value: Value::String(new.overflow.clone()),
		});
	}
	if old.white_space != new.white_space {
		sets.push(SetProp {
			key: PropKey::WhiteSpace,
			value: Value::String(new.white_space.clone()),
		});
	}

	if !sets.is_empty() {
		changes.push(ClientAction::SetProp {
			path: path.clone(),
			sets,
		});
	}

	// match (old, new) {
	//     (Item::View(old), Item::View(new)) => {
	//         log::trace!("{:?} inner_diff calculating view minumum edits", path);

	// 		// if old != new {
	// 		// 	println!("{:?} old and new are different", path);
	// 		// 	if old.background_color != new.background_color {
	// 		// 		println!("{:?} background color is different", path);
	// 		// 	}

	// 		// }

	// 		if old.spacing != new.spacing {
	// 			changes.push(ClientAction::SetStyle {
	// 				path: path.clone(),
	// 				prop: "gap".to_string(),
	// 				value: new.spacing.unwrap_or_default().to_string()
	// 			})
	// 		}
	//     }
	//     _ => {
	//         log::trace!("{:?} comparing old and new", path);

	//         if old != new {
	//             log::trace!("{:?} old and new are different", path);

	//             changes.push(
	//                 ClientAction::Replace(
	//                     Replace {
	//                         path: path.clone(),
	//                         item: new.clone()
	//                     }
	//                 )
	//             );
	//         }
	//     }
	// }
}

fn diff_children(
	changes: &mut Vec<ClientAction>,
	old_body: &Vec<Item>,
	new_body: &Vec<Item>,
	path: ItemPath,
) {
	let edits = get_minimum_edits(old_body, new_body);
	for edit in edits {
		match edit {
			EditOperation::InsertFirst(item) => {
				changes.push(ClientAction::AddFront(AddFront {
					path: path.clone(),
					item,
				}));
			}
			EditOperation::InsertAfter(index, item) => {
				changes.push(ClientAction::InsertAt(InsertAt {
					path: path.clone(),
					inx: index,
					item,
				}));
			}
			EditOperation::RemoveAt(index) => {
				changes.push(ClientAction::RemoveInx(RemoveInx {
					path: path.clone(),
					inx: index,
				}));
			}
			EditOperation::ReplaceAt(i, item) => {
				let mut child_path = path.clone();
				child_path.push(i);
				inner_diff(changes, &old_body[i], &item, child_path);
			}
			EditOperation::InsertBack(item) => {
				changes.push(ClientAction::AddBack(AddBack {
					path: path.clone(),
					item,
				}));
			}
		}
	}
}

pub fn diff(old: &Item, new: &Item) -> Vec<ClientAction> {
	log::trace!("diff");
	log::trace!("{:?}", old);
	log::trace!("{:?}", new);
	let mut changes = Vec::new();
	let path = Vec::new();
	inner_diff(&mut changes, old, new, path);
	log::debug!("diff changes: {:?}", changes);
	changes
}

#[cfg(test)]
mod tests {
	use crate::gui::hstack;

	#[test]
	fn test_view_metadata_diff() {
		let old = hstack([]);
		let new = hstack([]).spacing(10);

		let changes = super::diff(&old, &new);
		println!("{:?}", changes);
	}
}
