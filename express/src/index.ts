import dotenv from "dotenv";
dotenv.config();

import express from "express";
import Joi from "joi"

import { GroupChange, setGroup, updateColor } from "./hue";
import { startPeriodicUpdate } from "./colorManager";

interface ProcessEnv {
	EXPRESS_PORT: number,
}
const envSchema = Joi.object<ProcessEnv>({
	EXPRESS_PORT: Joi.number().min(0).required(),
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
	shouldActivate: boolean,
}
const updateColorSchema = Joi.object<UpdateColorBody>({
	mirek: Joi.number().min(153).max(500).required(),
	brightness: Joi.number().min(0).max(100).required(),
	shouldActivate: Joi.boolean().required(),
});
app.put('/update-color', async (req: express.Request, res: express.Response) => {
	try {
		const { mirek, brightness } = throwableValidation<UpdateColorBody>(req.body, updateColorSchema);
		await updateColor(mirek, brightness, 0);
		res.json({})
	} catch(error) {
		handleErrorResponse(error as Error, res);
	}
});

interface SetGroupBody {
	group: string,
	change: GroupChange,
}
const setGroupSchema = Joi.object<SetGroupBody>({
	group: Joi.string().required(),
	change: Joi.string().required(),
})
app.put('/set-group', async (req: express.Request, res: express.Response) => {
	try {
		const { group, change } = throwableValidation<SetGroupBody>(req.body, setGroupSchema);
		await setGroup(group, change);
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
