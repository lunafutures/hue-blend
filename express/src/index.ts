import dotenv from "dotenv";
dotenv.config();

import express from "express";
import Joi from "joi"

import { GroupChange, setGroup, updateColor } from "./hue";
import { startPeriodicUpdate } from "./colorManager";

interface ProcessEnv {
	EXPRESS_PORT: number,
	HUE_ALL_LIGHTS_GROUP_NAME: string,
}
const envSchema = Joi.object<ProcessEnv>({
	EXPRESS_PORT: Joi.number().min(0).required(),
	HUE_ALL_LIGHTS_GROUP_NAME: Joi.string().required(),
});
const { error, value: processEnv } = envSchema.validate(
	process.env, { allowUnknown: true});
if (error) {
	throw error;
}

function handleErrorResponse(error: Error, res: express.Response): void {
	res.statusCode = 400;
	res.json({ error: { name: error.name, message: error.message } });
}

function throwableValidation<T>(obj: Object, schema: Joi.ObjectSchema<T>): T {
	const { error, value } = schema.validate(obj);
	if (error) {
		throw error;
	}
	return value;
}

const app = express();
app.use(express.json());

interface UpdateColorBody {
	mirek: number,
	brightness: number,
}
const updateColorSchema = Joi.object<UpdateColorBody>({
	mirek: Joi.number().min(153).max(500).required(),
	brightness: Joi.number().min(0).max(100).required(),
});
app.put('/update-color', async (req: express.Request, res: express.Response) => {
	try {
		const { mirek, brightness } = throwableValidation<UpdateColorBody>(req.body, updateColorSchema);
		await updateColor(processEnv.HUE_ALL_LIGHTS_GROUP_NAME, mirek, brightness);
		res.json({})
	} catch(error) {
		handleErrorResponse(error as Error, res);
	}
});

interface SetGroupBody {
	groupName: string,
	change: GroupChange,
	mirek?: number,
	brightness?: number,
}
const setGroupSchema = Joi.object<SetGroupBody>({
	groupName: Joi.string().required(),
	change: Joi.string().required(),
	mirek: Joi.number().min(153).max(500),
	brightness: Joi.number().min(0).max(100),
});
app.put('/set-group', async (req: express.Request, res: express.Response) => {
	try {
		const { groupName, change, mirek, brightness } =
			throwableValidation<SetGroupBody>(req.body, setGroupSchema);
		await setGroup(groupName, change, mirek, brightness);
		res.json({});
	} catch(error) {
		handleErrorResponse(error as Error, res);
	}
});

const BRIGHT_MIREK = 233;
const BRIGHT_BRIGHTNESS = 100
const DARK_MIREK = 500;
const DARK_BRIGHTNESS = 50;

interface SimpleSetBody {
	groupName: string,
}
const simpleSetSchema = Joi.object<SimpleSetBody>({
	groupName: Joi.string().required(),
});
app.put('/set-bright', async (req: express.Request, res: express.Response) => {
	try {
		const { groupName } = throwableValidation<SimpleSetBody>(req.body, simpleSetSchema);
		await setGroup(groupName, GroupChange.ON, BRIGHT_MIREK, BRIGHT_BRIGHTNESS);
		res.json({});
	} catch(error) {
		handleErrorResponse(error as Error, res);
	}
});
app.put('/set-dark', async (req: express.Request, res: express.Response) => {
	try {
		const { groupName } = throwableValidation<SimpleSetBody>(req.body, simpleSetSchema);
		await setGroup(groupName, GroupChange.ON, DARK_MIREK, DARK_BRIGHTNESS);
		res.json({});
	} catch(error) {
		handleErrorResponse(error as Error, res);
	}
});

app.get('/', (_req: express.Request, res: express.Response) => {
	res.json({ up: true });
});

startPeriodicUpdate();

app.listen(processEnv.EXPRESS_PORT, () => {
	console.log(`hue-express app listening on port ${processEnv.EXPRESS_PORT}`);
});
