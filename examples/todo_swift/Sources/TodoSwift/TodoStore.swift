import Combine
import Foundation

@MainActor
public protocol WguiTodoBridge: AnyObject {
	var onSnapshot: ((TodoSnapshot) -> Void)? { get set }

	func start()
	func stop()
	func dispatch(_ action: TodoAction)
}

@MainActor
public final class TodoStore: ObservableObject {
	@Published public private(set) var snapshot: TodoSnapshot

	private let bridge: WguiTodoBridge

	public init(
		bridge: WguiTodoBridge,
		initialSnapshot: TodoSnapshot = TodoSnapshot()
	) {
		self.bridge = bridge
		self.snapshot = initialSnapshot
		self.bridge.onSnapshot = { [weak self] snapshot in
			self?.snapshot = snapshot
		}
	}

	public func start() {
		bridge.start()
	}

	public func stop() {
		bridge.stop()
	}

	public func send(_ action: TodoAction) {
		bridge.dispatch(action)
	}
}

@MainActor
public final class MockTodoBridge: WguiTodoBridge {
	public var onSnapshot: ((TodoSnapshot) -> Void)?

	private var nextId = 3
	private var snapshot = TodoSnapshot(
		draftTitle: "",
		items: [
			TodoItem(id: 1, title: "Write Rust store tests", completed: true),
			TodoItem(id: 2, title: "Render todo screen in SwiftUI", completed: false),
		]
	)

	public init() {}

	public func start() {
		publish()
	}

	public func stop() {}

	public func dispatch(_ action: TodoAction) {
		switch action.type {
		case "todo.draftChanged":
			snapshot.draftTitle = action.payload["title"] ?? ""
			snapshot.error = nil
		case "todo.add":
			addDraftTodo()
		case "todo.toggle":
			if let id = action.payload["id"].flatMap(Int.init) {
				toggle(id: id)
			}
		case "todo.delete":
			if let id = action.payload["id"].flatMap(Int.init) {
				snapshot.items.removeAll { $0.id == id }
			}
		case "todo.clearCompleted":
			snapshot.items.removeAll(where: \.completed)
		default:
			break
		}

		publish()
	}

	private func addDraftTodo() {
		let title = snapshot.draftTitle.trimmingCharacters(in: .whitespacesAndNewlines)
		guard !title.isEmpty else {
			snapshot.error = "Enter a todo before adding it."
			return
		}

		snapshot.items.append(TodoItem(id: nextId, title: title, completed: false))
		nextId += 1
		snapshot.draftTitle = ""
		snapshot.error = nil
	}

	private func toggle(id: Int) {
		guard let index = snapshot.items.firstIndex(where: { $0.id == id }) else {
			return
		}
		snapshot.items[index].completed.toggle()
	}

	private func publish() {
		onSnapshot?(snapshot)
	}
}

@MainActor
public final class JsonTodoBridge: WguiTodoBridge {
	public var onSnapshot: ((TodoSnapshot) -> Void)?

	private let startImpl: (@escaping (String) -> Void) -> Void
	private let stopImpl: () -> Void
	private let dispatchImpl: (String) -> Void
	private let decoder = JSONDecoder()
	private let encoder = JSONEncoder()

	public init(
		start: @escaping (@escaping (String) -> Void) -> Void,
		stop: @escaping () -> Void,
		dispatch: @escaping (String) -> Void
	) {
		self.startImpl = start
		self.stopImpl = stop
		self.dispatchImpl = dispatch
	}

	public func start() {
		startImpl { [weak self] rawSnapshot in
			Task { @MainActor in
				self?.receiveSnapshot(rawSnapshot)
			}
		}
	}

	public func stop() {
		stopImpl()
	}

	public func dispatch(_ action: TodoAction) {
		guard let data = try? encoder.encode(action),
		      let json = String(data: data, encoding: .utf8)
		else {
			return
		}

		dispatchImpl(json)
	}

	private func receiveSnapshot(_ rawSnapshot: String) {
		guard let data = rawSnapshot.data(using: .utf8),
		      let snapshot = try? decoder.decode(TodoSnapshot.self, from: data)
		else {
			return
		}

		onSnapshot?(snapshot)
	}
}
