use crate::edit_distance::get_minimum_edits;
use crate::edit_distance::EditOperation;
use crate::gui::Item;
use crate::gui::ItemPayload;
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

	if old.typ != new.typ {
		changes.push(ClientAction::Replace(Replace {
			path: path.clone(),
			item: new.clone()
		}));
		return;
	}

	let mut sets: Vec<SetProp> = Vec::new();

	match (&old.payload, &new.payload) {
		(ItemPayload::Layout(old), ItemPayload::Layout(new)) => {
			log::trace!("{:?} layout", path);

			if old.spacing != new.spacing {
				println!("{:?} spacing is different", path);
				sets.push(SetProp {
					key: PropKey::Spacing,
					value: Value::Number(new.spacing)
				})
			}

			let edits = get_minimum_edits(&old.body, &new.body);
			for edit in edits {
				match edit {
					EditOperation::InsertFirst(item) => {
						log::trace!("{:?} insert first", path);

						changes.push(
							ClientAction::AddFront(
								AddFront {
									path: path.clone(),
									item: item
								}
							)
						);
					},
					EditOperation::InsertAfter(index, item) => {
						log::trace!("{:?} insert after {}", path, index);

						changes.push(
							ClientAction::InsertAt(
								InsertAt {
									path: path.clone(),
									inx: index,
									item: item
								}
							)
						);
					},
					EditOperation::RemoveAt(index) => {
						log::trace!("{:?} remove at index {}", path, index);

						changes.push(
							ClientAction::RemoveInx(
								RemoveInx {
									path: path.clone(),
									inx: index
								}
							)
						);
					},
					EditOperation::ReplaceAt(i, item) => {
						log::trace!("{:?} replace at {}", path, i);

						let mut path = path.clone();
						path.push(i);

						log::trace!("{:?} new path: {:?}", path, path);

						inner_diff(changes, &old.body[i], &item, path);
					},
					EditOperation::InsertBack(item) => {
						log::trace!("{:?} insert back", path);

						todo!();
					}
				}
			}
		},
		_ => {
			if old != new {
                log::trace!("{:?} old and new are different", path);

                changes.push(
                    ClientAction::Replace(
                        Replace {
                            path: path.clone(),
                            item: new.clone()
                        }
                    )
                );
            }
		}
	}

	if old.id != new.id {
		sets.push(SetProp {
			key: PropKey::ID,
			value: Value::Number(new.id)
		})
	}

	if old.border != new.border {
		println!("{:?} border is different", path);
		sets.push(SetProp {
			key: PropKey::Border,
			value: Value::String(new.border.clone())
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
			value: Value::String(new.background_color.clone())
		});
	}

	if sets.len() > 0 {
		changes.push(ClientAction::SetProp {
			path: path.clone(),
			sets
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

pub fn diff(old: &Item, new: &Item) -> Vec<ClientAction> {
    log::trace!("diff");
    log::trace!("{:?}", old);
    log::trace!("{:?}", new);
    let mut changes = Vec::new();
    let mut path = Vec::new();
    inner_diff(&mut changes, old, new, path);
    log::debug!("diff changes: {:?}", changes);
    changes
}

#[cfg(test)]
mod tests {
    use crate::gui::hstack;
    use super::*;

	#[test]
	fn test_view_metadata_diff() {
		let old = hstack([]);
		let new = hstack([]).spacing(10);

		let changes = super::diff(&old.into(), &new.into());
		println!("{:?}", changes);
	}
}