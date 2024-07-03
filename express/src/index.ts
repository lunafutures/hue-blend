import dotenv from "dotenv";
dotenv.config();

import express from "express";
import Joi from "joi"

import { GroupChange, setGroup, updateColor } from "./hue";
import { startPeriodicUpdate } from "./colorManager";
import { State } from "./state";
import { EnvValidator } from "./envValidator";
import { logger } from "./logging";

interface ProcessEnv {
	EXPRESS_PORT: number,
	HUE_ALL_LIGHTS_GROUP_NAME: string,
}
const envSchema = Joi.object<ProcessEnv>({
	EXPRESS_PORT: Joi.number().min(0).required(),
	HUE_ALL_LIGHTS_GROUP_NAME: Joi.string().required(),
});
const env = new EnvValidator<ProcessEnv>(envSchema);

function handleErrorResponse(error: Error, res: express.Response): void {
	logger.error(`${error.message}`);
	res.status(400).json({ error: { name: error.name, message: error.message } });
}

function throwableValidation<T>(obj: object, schema: Joi.ObjectSchema<T>): T {
	const { error, value } = schema.validate(obj);
	if (error) {
		throw error;
	}
	return value;
}

const app = express();
app.use(express.json());

app.use((req: express.Request, _res: express.Response, next) => {
	logger.info(`${req.method} ${req.url}`);
	next();
});

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
		await updateColor(env.getProperty('HUE_ALL_LIGHTS_GROUP_NAME'), mirek, brightness);
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

app.get('/debug', async (_req: express.Request, res: express.Response) => {
	try {
		const state = await State.getInstance();
		res.json({
			now: new Date(),
			state,
		});
	} catch (error) {
		handleErrorResponse(error as Error, res);
	}
});

app.get('/', (_req: express.Request, res: express.Response) => {
	try {
		res.json({ up: true });
	} catch (error) {
		handleErrorResponse(error as Error, res);
	}
});

app.use((error: Error, _req: unknown, res: express.Response, _next: unknown): void => {
	handleErrorResponse(error, res);
});

startPeriodicUpdate();
app.listen(env.getProperty('EXPRESS_PORT'), () => {
	logger.info(`Express is listening on port ${env.getProperty('EXPRESS_PORT')}`);
});
