import Joi from "joi";
import axios from "axios";
import { updateColor } from "./hue";

interface ProcessEnv {
	RUST_HUE_URL: string;
}
const envSchema = Joi.object<ProcessEnv>({
	RUST_HUE_URL: Joi.string().required(),
});
const { error, value: processEnv } = envSchema.validate(
	process.env, { allowUnknown: true});
if (error) {
	throw error;
}

// XXX TODO tests
// {
//     "now": "2024-06-29T03:09:32.143386400-05:00",
//     "change_action": {
//         "color": {
//             "mirek": 500,
//             "brightness": 50
//         }
//     },
//     "just_updated": false
// }
// {
//     "now": "2024-06-29T07:52:48.862114700-05:00",
//     "change_action": "none",
//     "just_updated": true
// }

interface MirekBrightness {
	mirek: number,
	brightness: number,
}

interface ChangeActionColor {
	color: MirekBrightness,
}

interface NowChange {
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
const nowChangeSchema = Joi.object<NowChange>({
	now: Joi.string().required(),
	change_action: Joi.alternatives(Joi.string(), changeActionColorSchema).required(),
	just_updated: Joi.bool().required(),
});

async function getNowChange(): Promise<NowChange> {
	const response = await axios.get(`${processEnv.RUST_HUE_URL}/now`);
	const { error, value: nowChange } = nowChangeSchema.validate(response.data);
	if (error) {
		throw error;
	}

	return nowChange;
}

export async function getAndApplyChange() {
	let nowChange = await getNowChange();
	if (nowChange.change_action === "none" ) {
		return;
	}
	let changeColor = nowChange.change_action.color;
	await updateColor(changeColor.mirek, changeColor.brightness, false, 0);
}