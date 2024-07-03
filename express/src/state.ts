import _ from "lodash";
import { Group, GroupBody, IndividualDictionary, getGroupedLights, getLights, getRooms, getZones } from "./hue";

export class State {
	private zones: GroupBody | undefined;
	private rooms: GroupBody | undefined;
	private groups: IndividualDictionary;

	private static instance: State;

	private constructor() {
		this.groups = {};
	}

	private static async initialize(): Promise<State> {
		const instance = new State();
		await instance.populateGroups();
		return instance;
	}

	private async populateGroups() {
		const groupedLights = await getGroupedLights();
		const groupRidToId = _.fromPairs(_.map(groupedLights.data, item => [item.owner.rid, item.id]));

		this.zones = await getZones();
		this.zones.data.forEach(group => {
			this.groups[group.metadata.name] = group;
			const rid = this.groups[group.metadata.name].rid = group.id;
			const gid = this.groups[group.metadata.name].id = groupRidToId[rid];
			console.log(`Found zone: ${group.metadata.name} (rid: ${group.rid}, id: ${gid}).`);
		});

		this.rooms = await getRooms();
		this.rooms.data.forEach(group => {
			this.groups[group.metadata.name] = group;
			const rid = this.groups[group.metadata.name].rid = group.id;
			const gid = this.groups[group.metadata.name].id = groupRidToId[rid];
			console.log(`Found room: ${group.metadata.name} (rid: ${group.rid}, id: ${gid}).`);
		});
	}

	static async getInstance(): Promise<State> {
		if(!State.instance) {
			State.instance = await this.initialize();
		}

		return State.instance;
	}

	getGroup(groupName: string): Group {
		if (!this.groups.hasOwnProperty(groupName)) {
			throw new Error(`Group "${groupName}" not found in the object.`);
		}

		return this.groups[groupName];
	}
}
