import Foundation

public struct TodoItem: Codable, Equatable, Identifiable, Sendable {
	public var id: Int
	public var title: String
	public var completed: Bool

	public init(id: Int, title: String, completed: Bool) {
		self.id = id
		self.title = title
		self.completed = completed
	}
}

public struct TodoSnapshot: Codable, Equatable, Sendable {
	public var draftTitle: String
	public var items: [TodoItem]
	public var saving: Bool
	public var error: String?

	public init(
		draftTitle: String = "",
		items: [TodoItem] = [],
		saving: Bool = false,
		error: String? = nil
	) {
		self.draftTitle = draftTitle
		self.items = items
		self.saving = saving
		self.error = error
	}

	public var doneCount: Int {
		items.filter(\.completed).count
	}

	public var undoneCount: Int {
		items.count - doneCount
	}
}

public struct TodoAction: Codable, Equatable, Sendable {
	public var type: String
	public var payload: [String: String]

	public init(type: String, payload: [String: String] = [:]) {
		self.type = type
		self.payload = payload
	}

	public static func draftChanged(_ title: String) -> TodoAction {
		TodoAction(type: "todo.draftChanged", payload: ["title": title])
	}

	public static func add() -> TodoAction {
		TodoAction(type: "todo.add")
	}

	public static func toggle(id: Int) -> TodoAction {
		TodoAction(type: "todo.toggle", payload: ["id": String(id)])
	}

	public static func delete(id: Int) -> TodoAction {
		TodoAction(type: "todo.delete", payload: ["id": String(id)])
	}

	public static func clearCompleted() -> TodoAction {
		TodoAction(type: "todo.clearCompleted")
	}
}
