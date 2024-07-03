import fs from "fs";
import https from "https";
import tls from "tls";

import _ from "lodash";
import Joi from "joi";
import axios, { AxiosRequestConfig, AxiosResponse } from "axios";
import rateLimit from 'axios-rate-limit';
import { State } from "./state";

interface ProcessEnv {
	HUE_BRIDGE_BASE_URL: string;
	HUE_BRIDGE_CACERT_PEM_PATH: string;
	HUE_BRIDGE_API_KEY: string;
	HUE_BRIDGE_ID: string;
}
const envSchema = Joi.object<ProcessEnv>({
	HUE_BRIDGE_BASE_URL: Joi.string().required(),
	HUE_BRIDGE_CACERT_PEM_PATH: Joi.string().required(),
	HUE_BRIDGE_API_KEY: Joi.string().required(),
	HUE_BRIDGE_ID: Joi.string().required(),
});
const { error, value: processEnv } = envSchema.validate(
	process.env, { allowUnknown: true});
if (error) {
	throw error;
}

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
	color_temperature?: { mirek: number }, // between 153 and 500
	dimming?: { brightness: number },
	type: "light",
}

interface Error {
	description: string,
}

export interface Group {
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

interface LightBody {
	errors: Error[],
	data: LightData[],
}

export interface GroupBody {
	errors: Error[],
	data: Group[],
}

interface GenericBody {
	errors: Error[],
}

interface HueResponse<T> {
	data: T[]
	errors: Error[],
}

export type IndividualDictionary = {
	[key: string]: Group;
}

async function getGroups(url: string): Promise<GroupBody> {
	const response = await hueRequest({
		method: "get",
		url,
	});
	return response.data as GroupBody;
}

export function getZones() {
	return getGroups(`${processEnv.HUE_BRIDGE_BASE_URL}/clip/v2/resource/zone`);
}

export function getRooms() {
	return getGroups(`${processEnv.HUE_BRIDGE_BASE_URL}/clip/v2/resource/room`);
}

export async function getGroupedLights() {
	const response = await hueRequest({
		method: "get",
		url: `${processEnv.HUE_BRIDGE_BASE_URL}/clip/v2/resource/grouped_light`,
	});
	return response.data as HueResponse<GroupedLights>;
}

function getRids(group: Group): string[] {
	return _.map(group.children, resource => resource.rid);
}

function getLightsOnInGroup(lights: LightBody, rids: string[]): LightData[] {
	return _.filter(lights.data, (light: LightData) =>
		_.includes(rids, light.id) && light.on.on === true);
}

const ALLOWED_BRIGHTNESS_DIFF = 1.0;
function noLightsDeviate(lights: LightBody, rids: string[], desiredMirek: number, desiredBrightnessInexact: number): boolean {
	return _.every(getLightsOnInGroup(lights, rids), (onLight: LightData) => {
		const mirek = onLight.color_temperature?.mirek;
		if (mirek === undefined) {
			console.log("Deviation: mirek is undefined");
			return false;
		} else if (mirek != desiredMirek) {
			console.log(`Deviation: mirek differs: mirek (${mirek}) != desiredMirek (${desiredMirek})`);
			return false;
		}

		const brightness = onLight.dimming?.brightness;
		if (brightness === undefined) {
			console.log("Deviation: brightness is undefined");
			return false;
		} else if (Math.abs(brightness - desiredBrightnessInexact) > ALLOWED_BRIGHTNESS_DIFF) {
			console.log(`Deviation: brightness differs: brightness (${brightness}) ` +
				`!= desiredbrightnessInexact (${desiredBrightnessInexact}) by more than ${ALLOWED_BRIGHTNESS_DIFF}`);
			return false;
		}

		return true;
	});
}

export async function updateColor(groupName: string, mirek: number, brightness: number) {
	console.log(`\nUpdating color: mirek=${mirek} brightness=${brightness}`);

	const state = await State.getInstance();
	if (noLightsDeviate(await getLights(), getRids(state.getGroup(groupName)), mirek, brightness)) {
		console.log("Not updating lights because no lights deviate.");
		return;
	}

	const group = state.getGroup(groupName);
	const response = await hueRequest({
		method: "put",
		url: `${processEnv.HUE_BRIDGE_BASE_URL}/clip/v2/resource/grouped_light/${group.id}`,
		data: {
			type: "grouped_light",
			dimming: { brightness },
			color_temperature: { mirek },
		},
	});
	return response.data as GenericBody;
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

interface MirekBrightnessConfig {
	color_temperature?: { mirek: number },
	dimming?: { brightness: number },
}

function getMirekBrightnessConfig(mirek?: number, brightness?: number): MirekBrightnessConfig {
	return {
		...(mirek !== undefined ? { color_temperature: { mirek } } : {}),
		...(brightness !== undefined ? { dimming: { brightness } } : {}),
	};
}

export async function setGroup(groupName: string, change: GroupChange, mirek?: number, brightness?: number) {
	const state = await State.getInstance();
	const lightsOnInGroup = getLightsOnInGroup(
		await getLights(),
		getRids(state.getGroup(groupName)));
	const onOn = getOnOn(change, lightsOnInGroup.length);

	const changeAction = state.lastChange?.change_action;
	const mirekBrightnessConfig = changeAction === undefined || changeAction === "none"
		? getMirekBrightnessConfig(mirek, brightness)
		: getMirekBrightnessConfig(
			mirek ?? changeAction.color.mirek,
			brightness ?? changeAction.color.brightness);

	console.log(`Setting group "${groupName}": ${onOn} ${mirekBrightnessConfig}`);

	const group = state.getGroup(groupName);
	const response = await hueRequest({
		method: "put",
		url: `${processEnv.HUE_BRIDGE_BASE_URL}/clip/v2/resource/grouped_light/${group.id}`,
		data: {
			type: "grouped_light",
			...onOn,
			...mirekBrightnessConfig,
		},
	});
	return response.data as GenericBody;
}

export async function getLights(): Promise<LightBody> {
	const response = await hueRequest({
		method: "get",
		url: `${processEnv.HUE_BRIDGE_BASE_URL}/clip/v2/resource/light`,
	});
	return response.data as LightBody;
}

const rateLimitedHueRequester = (function() {
	// Allows not specifying "rejectUnauthorized: false"
	const httpsAgent = new https.Agent({
		ca: fs.readFileSync(processEnv.HUE_BRIDGE_CACERT_PEM_PATH),
		checkServerIdentity: (hostname, cert) => {
			const tlsResult = tls.checkServerIdentity(hostname, cert);
			if (tlsResult === undefined) {
				return undefined;
			}

			const error = tlsResult as unknown as Error & { code: string };
			if (error.code !== "ERR_TLS_CERT_ALTNAME_INVALID") {
				return tlsResult;
			}

			if (cert.subject.CN === processEnv.HUE_BRIDGE_ID.toLowerCase()) {
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

function hueRequest<T, D>(config: Partial<AxiosRequestConfig<D>>): Promise<AxiosResponse<T, D>> {
	config.headers = {
		"hue-application-key": processEnv.HUE_BRIDGE_API_KEY,
		...config.headers,
	};

	return rateLimitedHueRequester.request({
		...config,
	});
}
