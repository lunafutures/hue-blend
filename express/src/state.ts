import _ from "lodash";
import { Group, GroupBody, IndividualDictionary, getGroupedLights, getRooms, getZones } from "./hue";
import { NowChange } from "./colorManager";
import { logger } from "./logging";

export class State {
	private zones?: GroupBody;
	private rooms?: GroupBody;
	private groups: IndividualDictionary;

	private _lastChange?: NowChange;

	private static instancePromise: Promise<State>;

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
			this.groups[group.metadata.name].id = groupRidToId[rid];
			logger.debug(`Found zone: ${group.metadata.name}.`);
		});

		this.rooms = await getRooms();
		this.rooms.data.forEach(group => {
			this.groups[group.metadata.name] = group;
			const rid = this.groups[group.metadata.name].rid = group.id;
			this.groups[group.metadata.name].id = groupRidToId[rid];
			logger.debug(`Found room: ${group.metadata.name}.`);
		});
	}

	static async getInstance(): Promise<State> {
		if(!State.instancePromise) {
			State.instancePromise = this.initialize();
		}

		return State.instancePromise;
	}

	get lastChange(): NowChange | undefined {
		return this._lastChange;
	}

	set lastChange(value: NowChange) {
		this._lastChange = value;
	}

	getGroup(groupName: string): Group {
		if (!Object.prototype.hasOwnProperty.call(this.groups, groupName)) {
			throw new Error(`Group "${groupName}" not found in the object.`);
		}

		return this.groups[groupName];
	}
}
