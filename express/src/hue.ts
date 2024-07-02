import fs from "fs";
import https from "https";
import tls from "tls";

import _ from "lodash";
import Joi from "joi";
import axios, { AxiosRequestConfig, AxiosResponse } from "axios";
import rateLimit from 'axios-rate-limit';

interface ProcessEnv {
	HUE_BRIDGE_BASE_URL: string;
	HUE_BRIDGE_CACERT_PEM_PATH: string;
	HUE_BRIDGE_API_KEY: string;
	HUE_BRIDGE_ID: string;
	AUTOMATION_SCENE_ID: string;
}
const envSchema = Joi.object<ProcessEnv>({
	HUE_BRIDGE_BASE_URL: Joi.string().required(),
	HUE_BRIDGE_CACERT_PEM_PATH: Joi.string().required(),
	HUE_BRIDGE_API_KEY: Joi.string().required(),
	HUE_BRIDGE_ID: Joi.string().required(),
	AUTOMATION_SCENE_ID: Joi.string().required(),
});
const { error, value: processEnv } = envSchema.validate(
	process.env, { allowUnknown: true});
if (error) {
	throw error;
}
const hueBridgeBaseUrl = processEnv.HUE_BRIDGE_BASE_URL;
const hueBridgeCertPath = processEnv.HUE_BRIDGE_CACERT_PEM_PATH;
const apiKey = processEnv.HUE_BRIDGE_API_KEY;
const bridgeId = processEnv.HUE_BRIDGE_ID;
const automationSceneId = processEnv.AUTOMATION_SCENE_ID;

type Unimplemented = unknown;

interface Resource {
	rid: string,
	rtype: string,
}

interface Metadata {
	name: string,
	archetype: string,
}

interface LightData {
	id: string,
	id_v1: string,
	owner: Resource,
	metadata: Metadata,
	on: { on: boolean },
	type: "light",
}

interface Error {
	description: String,
}

interface Group {
	id: string,
	rid: string,
	children: Resource[],
	metadata: Metadata,
	type: string
}

interface GroupedLights {
	id: string,
	id_v1: string,
	owner: Resource,
}

interface Action {
	on: { on: boolean },
	dimming?: { brightness: number },
	color?: { xy: { x: number, y: number }},
	color_temperature?: { mirek: number }, // between 153 and 500
	gradient?: Unimplemented, 
	effects?: Unimplemented,
}

interface ActionGet {
	target: Resource,
	action: Action,
}

interface SceneGet {
	id: string,
	id_v1: string,
	actions: ActionGet[],
	metadata: Metadata,
	group: Resource,
	palette: Unimplemented,
	speed: Unimplemented,
	auto_dynamic: Unimplemented,
	status: { active: "inactive" | "static" | "dynamic_palette" },
}

interface LightBody {
	errors: Error[],
	data: LightData[],
}

interface GroupBody {
	errors: Error[],
	data: Group[],
}

interface SceneBody {
	errors: Error[],
	data: SceneGet[],
}

interface GenericBody {
	errors: Error[],
}

interface HueResponse<T> {
	data: T[]
	errors: Error[],
}

type IndividualDictionary = {
	[key: string]: Group;
}

export class State {
	private zones: GroupBody | undefined;
	private rooms: GroupBody | undefined;
	private groups: IndividualDictionary;

	constructor() {
		this.groups = {};
		this.populateGroups();
	}

	async populateGroups() {
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

	getGroup(groupName: string): Group {
		if (!this.groups.hasOwnProperty(groupName)) {
			throw new Error(`Group "${groupName}" not found in the object.`);
		}

		return this.groups[groupName];
	}

	async testLights() {
		const lights = await getLights();
		lights.data.forEach((light, i) => {
			console.log(i, light.metadata.name, light.on.on)
		});
	}

	async test() {
		console.log("groups:", this.groups);
		Object.keys(this.groups).forEach(key => {
			const group = this.groups[key];
			console.log(group.metadata.name, group.children.length);
		});
	}
}
export async function createState(): Promise<State> {
	const state = new State();
	await state.populateGroups();
	return state;
}

async function getGroups(url: string): Promise<GroupBody> {
	const response = await hueRequest({
		method: "get",
		url,
	});
	return response.data as GroupBody;
}

function getZones() {
	return getGroups(`${hueBridgeBaseUrl}/clip/v2/resource/zone`);
}

function getRooms() {
	return getGroups(`${hueBridgeBaseUrl}/clip/v2/resource/room`);
}

async function getGroupedLights() {
	const response = await hueRequest({
		method: "get",
		url: `${hueBridgeBaseUrl}/clip/v2/resource/grouped_light`,
	});
	return response.data as HueResponse<GroupedLights>;
}

function getRids(group: Group): string[] {
	return _.map(group.children, resource => resource.rid);
}

function getLightsOn(lights: LightBody): _.Collection<string> {
	return _(lights.data)
		.filter((light: LightData) => light.on.on)
		.map(light => light.id);
}

function getLightsOnInGroup(lights: LightBody, rids: string[]): LightData[] {
	return _.filter(lights.data, (light: LightData) =>
		_.includes(rids, light.id) && light.on.on === true);
}

export async function updateColor(groupName: string, mirek: number, brightness: number) {
	console.log(`Updating color: mirek=${mirek} brightness=${brightness}`);
	// TODO XXX: Check grouped light color before updating
	
	const group = (await state).getGroup(groupName);
	const response = await hueRequest({
		method: "put",
		url: `${hueBridgeBaseUrl}/clip/v2/resource/grouped_light/${group.id}`,
		data: {
			type: "grouped_light",
			dimming: { brightness },
			color_temperature: { mirek },
		},
	});
	return response.data as GenericBody;
}

export async function updateColorScene(mirek: number, brightness: number, duration: number) {
	console.log(`Updating color, mirek: ${mirek}, brightness: ${brightness}`);
	const lights = await getLights();
	const lightsOnRids = getLightsOn(lights);

	const automationScene = await getAutomationScene();
	automationScene.data = automationScene.data.map(scene => {
		scene.actions = scene.actions.map(action => {
			action.action.on.on = lightsOnRids.includes(action.target.rid);
			action.action.color_temperature = { mirek };
			action.action.dimming = { brightness: 100 };
			return action;
		});
		return scene;
	});
	for (const sceneGet of automationScene.data) {
		const requestData = { actions: sceneGet.actions };
		await updateAutomationScene(requestData);
	}

	await activateAutomationScene(duration, brightness);
}

export enum GroupChange {
	ON = "on",
	OFF = "off",
	TOGGLE = "toggle",
	NONE = "none",
}

interface OnOn {
	on?: { on: boolean },
}

function getOnOn(groupChange: GroupChange, numLightsOn: number): OnOn {
	switch(groupChange) {
		case GroupChange.ON: return { on: { on: true } };
		case GroupChange.OFF: return { on: { on: false } };
		case GroupChange.TOGGLE: return { on: { on: numLightsOn === 0 } };
		case GroupChange.NONE: return {};
		default:
			throw(new Error(`Unexpected change type: ${groupChange}`));
	}
}

export async function setGroup(groupName: string, change: GroupChange, mirek?: number, brightness?: number) {
	const lightsOnInGroup = getLightsOnInGroup(
		await getLights(),
		getRids((await state).getGroup(groupName)));
	let onOn = getOnOn(change, lightsOnInGroup.length);

	let mirekConfig = mirek === undefined ? {} :
		{ color_temperature: { mirek }};
	let brightnessConfig = brightness === undefined ? {} :
		{ dimming: { brightness }};

	console.log(`Setting group "${groupName}": ${onOn} ${brightnessConfig} ${mirekConfig}`);

	const group = (await state).getGroup(groupName);
	const response = await hueRequest({
		method: "put",
		url: `${hueBridgeBaseUrl}/clip/v2/resource/grouped_light/${group.id}`,
		data: {
			type: "grouped_light",
			...onOn,
			...mirekConfig,
			...brightnessConfig,
		},
	});
	return response.data as GenericBody;
}

export async function setGroupScene(groupName: string, change: GroupChange) {
	const groupLights = (await state).getGroup(groupName);
	const groupRids = getRids(groupLights);
	const lights = await getLights();
	const lightsOnInGroup = getLightsOnInGroup(lights, groupRids);
	let turnOn = false;
	switch(change) {
		case GroupChange.ON:
			turnOn = true;
			break;
		case GroupChange.OFF:
			turnOn = false;
			break;
		case GroupChange.TOGGLE:
			turnOn = lightsOnInGroup.length === 0;
			break;
		default:
			throw(new Error(`Unexpected change type: ${change}`));
	}
	console.log(`Toggling group "${groupName}" to ${(turnOn? "on": "off")}`);

	const automationScene = await getAutomationScene();
	automationScene.data = automationScene.data.map(scene => {
		scene.actions = scene.actions.map(action => {
			if (_.includes(groupRids, action.target.rid)) {
				action.action.on.on = turnOn;
			}
			// action.action.color_temperature = { mirek: 233 }; // Not necessary to update
			// action.action.dimming = { brightness: 100 }

			return action;
		});
		return scene;
	});
	for (const sceneGet of automationScene.data) {
		const requestData = { actions: sceneGet.actions };
		await updateAutomationScene(requestData);
	}
	await activateAutomationScene(1000, 100);
}

export async function getLights(): Promise<LightBody> {
	const response = await hueRequest({
		method: "get",
		url: `${hueBridgeBaseUrl}/clip/v2/resource/light`,
	});
	return response.data as LightBody;
}

export async function getAutomationScene(): Promise<SceneBody> {
	const response = await hueRequest({
		method: "get",
		url: `${hueBridgeBaseUrl}/clip/v2/resource/scene/${automationSceneId}`,
	});
	return response.data as SceneBody;
}

export async function updateAutomationScene(sceneGet: Unimplemented): Promise<GenericBody> {
	const response = await hueRequest({
		method: "put",
		url: `${hueBridgeBaseUrl}/clip/v2/resource/scene/${automationSceneId}`,
		data: sceneGet,
	});
	return response.data as GenericBody;
}

export async function activateAutomationScene(duration: number, brightness: number): Promise<GenericBody> {
	const response = await hueRequest({
		method: "put",
		url: `${hueBridgeBaseUrl}/clip/v2/resource/scene/${automationSceneId}`,
		data: {
			recall: {
				action: "active",
				duration, // Gradient transition time in ms
				dimming: {
					brightness, // Scene global brightness
				},
			},
		},
	});
	return response.data as GenericBody;
}

const rateLimitedHueRequester = (function getRatedLimitedAxios<D>() {
	// Allows not specifying "rejectUnauthorized: false"
	const httpsAgent = new https.Agent({
		ca: fs.readFileSync(hueBridgeCertPath),
		checkServerIdentity: (hostname, cert) => {
			const tlsResult = tls.checkServerIdentity(hostname, cert);
			if (tlsResult === undefined) {
				return undefined;
			}

			const error = tlsResult as unknown as Error & { code: string };
			if (error.code !== "ERR_TLS_CERT_ALTNAME_INVALID") {
				return tlsResult;
			}

			if (cert.subject.CN === bridgeId.toLowerCase()) {
				return undefined;
			}
			
			const errorMessage = `Unexpected common name: ${cert.subject.CN}`;
			return Error(errorMessage);
		},
	});

	return rateLimit(axios.create({
		httpsAgent,
	}), { maxRequests: 1, perMilliseconds: 100 });
})();

function hueRequest<D>(config: Partial<AxiosRequestConfig<D>>): Promise<AxiosResponse<any, any>> {
	config.headers = {
		"hue-application-key": apiKey,
		...config.headers,
	};

	return rateLimitedHueRequester.request({
		...config,
	});
}

const state = createState();
