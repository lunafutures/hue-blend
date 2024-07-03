
import Joi from "joi";
import axios from "axios";
import cron from "node-cron";
import { updateColor } from "./hue";
import { State } from "./state";
import { EnvValidator } from "./envValidator";
import { logger } from "./logging";

interface ProcessEnv {
	RUST_HUE_URL: string,
	PERIODIC_UPDATE_CRON_STRING: string,
	HUE_ALL_LIGHTS_GROUP_NAME: string,
}
const env = new EnvValidator<ProcessEnv>(Joi.object<ProcessEnv>({
	RUST_HUE_URL: Joi.string().required(),
	PERIODIC_UPDATE_CRON_STRING: Joi.string().required(),
	HUE_ALL_LIGHTS_GROUP_NAME: Joi.string().required(),
}));

interface MirekBrightness {
	mirek: number,
	brightness: number,
}

interface ChangeActionColor {
	color: MirekBrightness,
}

export interface NowChange {
	now: string,
	change_action: ChangeActionColor | "none",
	just_updated: boolean,
}

const mirekBrightnessSchema = Joi.object<MirekBrightness>({
	mirek: Joi.number().required(),
	brightness: Joi.number().required(),
});
const changeActionColorSchema = Joi.object<ChangeActionColor>({
	color: mirekBrightnessSchema,
});
export const nowChangeSchema = Joi.object<NowChange>({
	now: Joi.string().required(),
	change_action: Joi.alternatives(Joi.string(), changeActionColorSchema).required(),
	just_updated: Joi.bool().required(),
});

async function getNowChange(): Promise<NowChange> {
	const response = await axios.get(`${env.getProperty('RUST_HUE_URL')}/now`);
	const { error, value: nowChange } = nowChangeSchema.validate(response.data);
	if (error) {
		throw error;
	}

	return nowChange;
}

async function getAndApplyChange() {
	logger.info("Applying periodic change...");
	const nowChange = await getNowChange();

	const state = await State.getInstance();
	state.lastChange = nowChange;

	if (nowChange.change_action === "none" ) {
		logger.debug("Change action is none. Nothing to change.")
		return;
	}
	const changeColor = nowChange.change_action.color;
	await updateColor(
		env.getProperty('HUE_ALL_LIGHTS_GROUP_NAME'),
		changeColor.mirek,
		changeColor.brightness);
}

export function startPeriodicUpdate() {
	cron.schedule(env.getProperty('PERIODIC_UPDATE_CRON_STRING'), () => {
		getAndApplyChange();
	});
}