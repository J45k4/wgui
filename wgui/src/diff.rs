use crate::edit_distance::get_minimum_edits;
use crate::edit_distance::EditOperation;
use crate::gui::Item;
use crate::types::AddFront;
use crate::types::ClientAction;
use crate::types::InsertAt;
use crate::types::ItemPath;
use crate::types::RemoveInx;
use crate::types::Replace;

fn inner_diff(changes: &mut Vec<ClientAction>, old: &Item, new: &Item, path: ItemPath) {
    log::trace!("{:?} inner_dif", path);

    match (old, new) {
        (Item::View(old), Item::View(new)) => {
            log::trace!("{:?} inner_diff calculating view minumum edits", path);

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
        }
        _ => {
            log::trace!("{:?} comparing old and new", path);

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