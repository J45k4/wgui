import SwiftUI

public struct TodoView: View {
	@StateObject private var store: TodoStore

	@MainActor
	public init(bridge: WguiTodoBridge = MockTodoBridge()) {
		_store = StateObject(wrappedValue: TodoStore(bridge: bridge))
	}

	public var body: some View {
		NavigationStack {
			List {
				Section {
					HStack {
						TextField(
							"What needs to be done?",
							text: Binding(
								get: { store.snapshot.draftTitle },
								set: { store.send(.draftChanged($0)) }
							)
						)
						.submitLabel(.done)
						.onSubmit {
							store.send(.add())
						}

						Button("Add") {
							store.send(.add())
						}
						.disabled(store.snapshot.draftTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
					}

					if let error = store.snapshot.error {
						Text(error)
							.foregroundStyle(.red)
					}
				}

				Section {
					ForEach(store.snapshot.items) { item in
						Toggle(
							isOn: Binding(
								get: { item.completed },
								set: { _ in store.send(.toggle(id: item.id)) }
							)
						) {
							Text(item.title)
								.strikethrough(item.completed)
								.foregroundStyle(item.completed ? .secondary : .primary)
						}
					}
					.onDelete { offsets in
						for index in offsets {
							let item = store.snapshot.items[index]
							store.send(.delete(id: item.id))
						}
					}
				} header: {
					Text("Todos")
				} footer: {
					Text("\(store.snapshot.doneCount) done / \(store.snapshot.undoneCount) undone")
				}
			}
			.navigationTitle("Todo List")
			.toolbar {
				if store.snapshot.doneCount > 0 {
					Button("Clear Done") {
						store.send(.clearCompleted())
					}
				}
			}
			.overlay {
				if store.snapshot.saving {
					ProgressView()
				}
			}
			.task {
				store.start()
			}
			.onDisappear {
				store.stop()
			}
		}
	}
}

#Preview {
	TodoView()
}
