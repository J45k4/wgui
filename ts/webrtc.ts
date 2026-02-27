import { MessageSender } from "./message_sender.ts"
import { SrvMessage } from "./types.ts"

type RoomElements = {
	localVideo: HTMLVideoElement[]
	localAudio: HTMLAudioElement[]
	remoteVideo: HTMLVideoElement[]
	remoteAudio: HTMLAudioElement[]
}

type RoomState = {
	joined: boolean
	selfClientId?: number
	peers: number[]
	wantsLocalAudio: boolean
	wantsLocalVideo: boolean
	localStream?: MediaStream
	peerConnections: Map<number, RTCPeerConnection>
	remoteStreams: Map<number, MediaStream>
	pendingIceCandidates: Map<number, RTCIceCandidateInit[]>
}

const ICE_SERVERS: RTCIceServer[] = [{ urls: ["stun:stun.l.google.com:19302"] }]

export class WebRtcCoordinator {
	private sender: MessageSender
	private rooms = new Map<string, RoomState>()

	constructor(sender: MessageSender) {
		this.sender = sender
	}

	public onSocketOpen() {
		for (const roomState of this.rooms.values()) {
			roomState.joined = false
		}
	}

	public syncElements(root: HTMLElement) {
		const elementsByRoom = this.collectRoomElements(root)
		const desiredRooms = new Set(Object.keys(elementsByRoom))

		for (const [room, roomState] of this.rooms.entries()) {
			if (!desiredRooms.has(room)) {
				this.leaveRoom(room, roomState)
				this.rooms.delete(room)
			}
		}

		for (const room of desiredRooms) {
			const elements = elementsByRoom[room]
			let state = this.rooms.get(room)
			if (!state) {
				state = {
					joined: false,
					peers: [],
					wantsLocalAudio: false,
					wantsLocalVideo: false,
					peerConnections: new Map(),
					remoteStreams: new Map(),
					pendingIceCandidates: new Map(),
				}
				this.rooms.set(room, state)
			}

			const wantsLocalVideo = elements.localVideo.length > 0
			state.wantsLocalVideo = wantsLocalVideo
			// Video calls should carry microphone audio as well.
			state.wantsLocalAudio = elements.localAudio.length > 0 || wantsLocalVideo

			this.applyLocalPreview(state, elements)
			this.applyRemoteMedia(state, elements)

			if (!state.joined) {
				this.sender.sendImmediate({
					type: "webRtcJoin",
					room,
					audio: state.wantsLocalAudio,
					video: state.wantsLocalVideo,
				})
				state.joined = true
			}

			if (state.joined) {
				this.ensureLocalMedia(room, state)
			}
		}
	}

	public handleServerMessage(message: SrvMessage) {
		if (message.type === "webRtcRoomState") {
			const roomState = this.rooms.get(message.room)
			if (!roomState) {
				return
			}
			roomState.selfClientId = message.selfClientId
			roomState.peers = message.peers
			this.reconcilePeers(message.room, roomState)
			return
		}

		if (message.type === "webRtcSignal") {
			const roomState = this.rooms.get(message.room)
			if (!roomState) {
				return
			}
			this.handleSignal(message.room, roomState, message.fromClientId, message.payload)
		}
	}

	private leaveRoom(room: string, roomState: RoomState) {
		for (const pc of roomState.peerConnections.values()) {
			pc.close()
		}
		roomState.peerConnections.clear()
		roomState.remoteStreams.clear()
		roomState.pendingIceCandidates.clear()
		roomState.localStream?.getTracks().forEach((track) => track.stop())
		roomState.localStream = undefined
		if (roomState.joined) {
			this.sender.sendImmediate({
				type: "webRtcLeave",
				room,
			})
		}
	}

	private collectRoomElements(root: HTMLElement): Record<string, RoomElements> {
		const out: Record<string, RoomElements> = {}
		const rtcEls = root.querySelectorAll("[data-wgui-rtc-room]")
		for (const el of rtcEls) {
			if (!(el instanceof HTMLMediaElement)) {
				continue
			}
			const room = el.dataset.wguiRtcRoom || ""
			if (!room) {
				continue
			}
			if (!out[room]) {
				out[room] = {
					localVideo: [],
					localAudio: [],
					remoteVideo: [],
					remoteAudio: [],
				}
			}
			const isLocal = el.dataset.wguiRtcLocal === "1"
			const kind = el.dataset.wguiRtc
			if (kind === "video") {
				if (isLocal && el instanceof HTMLVideoElement) {
					out[room].localVideo.push(el)
				} else if (el instanceof HTMLVideoElement) {
					out[room].remoteVideo.push(el)
				}
			}
			if (kind === "audio") {
				if (isLocal && el instanceof HTMLAudioElement) {
					out[room].localAudio.push(el)
				} else if (el instanceof HTMLAudioElement) {
					out[room].remoteAudio.push(el)
				}
			}
		}
		return out
	}

	private async ensureLocalMedia(room: string, roomState: RoomState) {
		const wantsMedia = roomState.wantsLocalAudio || roomState.wantsLocalVideo
		if (!wantsMedia || roomState.localStream) {
			return
		}
		try {
			roomState.localStream = await navigator.mediaDevices.getUserMedia({
				audio: roomState.wantsLocalAudio,
				video: roomState.wantsLocalVideo,
			})
			const peersNeedingRenegotiation: number[] = []
			for (const [peerId, pc] of roomState.peerConnections.entries()) {
				this.addLocalTracks(pc, roomState.localStream)
				// If tracks are attached after the initial handshake, force one renegotiation.
				if (
					pc.signalingState === "stable" &&
					pc.localDescription &&
					pc.remoteDescription
				) {
					peersNeedingRenegotiation.push(peerId)
				}
			}
			for (const peerId of peersNeedingRenegotiation) {
				await this.createOffer(room, roomState, peerId)
			}
			this.syncElements(document.body)
		} catch (err) {
			console.error("failed to getUserMedia for room", room, err)
		}
	}

	private reconcilePeers(room: string, roomState: RoomState) {
		const activePeers = new Set(roomState.peers.filter((id) => id !== roomState.selfClientId))

		for (const [peerId, pc] of roomState.peerConnections.entries()) {
			if (activePeers.has(peerId)) {
				continue
			}
			pc.close()
			roomState.peerConnections.delete(peerId)
			roomState.remoteStreams.delete(peerId)
			roomState.pendingIceCandidates.delete(peerId)
		}

		for (const peerId of activePeers) {
			const existing = roomState.peerConnections.get(peerId)
			if (existing) {
				continue
			}
			const pc = this.createPeerConnection(room, roomState, peerId)
			roomState.peerConnections.set(peerId, pc)
			if ((roomState.selfClientId ?? 0) < peerId) {
				this.createOffer(room, roomState, peerId)
			}
		}

		this.syncElements(document.body)
	}

	private createPeerConnection(room: string, roomState: RoomState, peerId: number): RTCPeerConnection {
		const pc = new RTCPeerConnection({ iceServers: ICE_SERVERS })
		if (roomState.localStream) {
			this.addLocalTracks(pc, roomState.localStream)
		}

		pc.onicecandidate = (event) => {
			if (!event.candidate) {
				return
			}
			this.sender.sendImmediate({
				type: "webRtcSignal",
				room,
				targetClientId: peerId,
				payload: JSON.stringify({ kind: "ice", candidate: event.candidate }),
			})
		}

		pc.ontrack = (event) => {
			const stream = event.streams[0] ?? this.ensurePeerRemoteStream(roomState, peerId)
			if (!event.streams[0]) {
				stream.addTrack(event.track)
			}
			roomState.remoteStreams.set(peerId, stream)
			this.syncElements(document.body)
		}

		pc.onconnectionstatechange = () => {
			if (pc.connectionState === "failed" || pc.connectionState === "closed") {
				roomState.remoteStreams.delete(peerId)
				roomState.pendingIceCandidates.delete(peerId)
				this.syncElements(document.body)
			}
		}

		return pc
	}

	private addLocalTracks(pc: RTCPeerConnection, stream: MediaStream) {
		for (const track of stream.getTracks()) {
			pc.addTrack(track, stream)
		}
	}

	private async createOffer(room: string, roomState: RoomState, peerId: number) {
		const pc = roomState.peerConnections.get(peerId)
		if (!pc) {
			return
		}
		await this.ensureLocalMedia(room, roomState)
		if (!roomState.localStream) {
			this.ensureReceiveTransceivers(pc, roomState)
		}
		const offer = await pc.createOffer()
		await pc.setLocalDescription(offer)
		this.sender.sendImmediate({
			type: "webRtcSignal",
			room,
			targetClientId: peerId,
			payload: JSON.stringify({ kind: "offer", sdp: offer }),
		})
	}

	private async handleSignal(room: string, roomState: RoomState, fromClientId: number, payload: string) {
		let signal: any
		try {
			signal = JSON.parse(payload)
		} catch (_) {
			return
		}

		let pc = roomState.peerConnections.get(fromClientId)
		if (!pc) {
			pc = this.createPeerConnection(room, roomState, fromClientId)
			roomState.peerConnections.set(fromClientId, pc)
		}

		if (signal.kind === "offer") {
			await this.ensureLocalMedia(room, roomState)
			if (!roomState.localStream) {
				this.ensureReceiveTransceivers(pc, roomState)
			}
			if (pc.signalingState !== "stable") {
				return
			}
			await pc.setRemoteDescription(new RTCSessionDescription(signal.sdp))
			await this.flushPendingIce(roomState, fromClientId, pc)
			const answer = await pc.createAnswer()
			await pc.setLocalDescription(answer)
			this.sender.sendImmediate({
				type: "webRtcSignal",
				room,
				targetClientId: fromClientId,
				payload: JSON.stringify({ kind: "answer", sdp: answer }),
			})
			return
		}

		if (signal.kind === "answer") {
			if (pc.signalingState !== "have-local-offer" || !pc.localDescription) {
				return
			}
			await pc.setRemoteDescription(new RTCSessionDescription(signal.sdp))
			await this.flushPendingIce(roomState, fromClientId, pc)
			return
		}

		if (signal.kind === "ice" && signal.candidate) {
			if (!pc.remoteDescription) {
				const queued = roomState.pendingIceCandidates.get(fromClientId) ?? []
				queued.push(signal.candidate)
				roomState.pendingIceCandidates.set(fromClientId, queued)
				return
			}
			await pc.addIceCandidate(new RTCIceCandidate(signal.candidate))
		}
	}

	private applyLocalPreview(roomState: RoomState, elements: RoomElements) {
		for (const video of elements.localVideo) {
			video.srcObject = roomState.localStream ?? null
			this.ensurePlayback(video)
		}
		for (const audio of elements.localAudio) {
			audio.srcObject = roomState.localStream ?? null
			this.ensurePlayback(audio)
		}
	}

	private applyRemoteMedia(roomState: RoomState, elements: RoomElements) {
		const stream = roomState.remoteStreams.values().next().value as MediaStream | undefined
		for (const video of elements.remoteVideo) {
			video.srcObject = stream ?? null
			this.ensurePlayback(video)
		}
		for (const audio of elements.remoteAudio) {
			audio.srcObject = stream ?? null
			this.ensurePlayback(audio)
		}
	}

	private ensurePlayback(element: HTMLMediaElement) {
		if (!element.autoplay || !element.srcObject) {
			return
		}
		const promise = element.play()
		if (promise && typeof promise.catch === "function") {
			promise.catch(() => {})
		}
	}

	private ensureReceiveTransceivers(pc: RTCPeerConnection, roomState: RoomState) {
		const hasKind = (kind: "audio" | "video") =>
			pc
				.getTransceivers()
				.some((transceiver) => transceiver.receiver.track?.kind === kind || transceiver.sender.track?.kind === kind)

		if (roomState.wantsLocalAudio && !hasKind("audio")) {
			pc.addTransceiver("audio", { direction: "recvonly" })
		}
		if (roomState.wantsLocalVideo && !hasKind("video")) {
			pc.addTransceiver("video", { direction: "recvonly" })
		}
	}

	private ensurePeerRemoteStream(roomState: RoomState, peerId: number) {
		const existing = roomState.remoteStreams.get(peerId)
		if (existing) {
			return existing
		}
		const stream = new MediaStream()
		roomState.remoteStreams.set(peerId, stream)
		return stream
	}

	private async flushPendingIce(roomState: RoomState, peerId: number, pc: RTCPeerConnection) {
		const queued = roomState.pendingIceCandidates.get(peerId)
		if (!queued || queued.length === 0 || !pc.remoteDescription) {
			return
		}
		roomState.pendingIceCandidates.delete(peerId)
		for (const candidate of queued) {
			try {
				await pc.addIceCandidate(new RTCIceCandidate(candidate))
			} catch (err) {
				console.warn("failed queued ICE candidate", err)
			}
		}
	}
}
