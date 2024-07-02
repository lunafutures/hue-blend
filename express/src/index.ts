import dotenv from "dotenv";
dotenv.config();

import cron from "node-cron";
import express from "express";
import Joi from "joi"

import { GroupChange, createState, toggleGroup, updateColor } from "./hue";
import { getAndApplyChange } from "./colorManager";

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

interface ToggleGroupBody {
	group: string,
	change: GroupChange,
}
const toggleGroupSchema = Joi.object<ToggleGroupBody>({
	group: Joi.string().required(),
	change: Joi.string().required(),
})
app.put('/toggle-group', async (req: express.Request, res: express.Response) => {
	try {
		const { group, change } = throwableValidation<ToggleGroupBody>(req.body, toggleGroupSchema);
		await toggleGroup(await createState(), group, change);
		res.json({});
	} catch(error) {
		handleErrorResponse(error as Error, res);
	}
});

app.get('/status', (req: express.Request, res: express.Response) => {
	res.json({ status: "up" });
});

cron.schedule('*/10 * * * * *', () => {
	console.log(`${new Date()} running a task every so and so.`);
	getAndApplyChange();
});

app.listen(process.env.EXPRESS_PORT, () => {
	console.log(`hue-express app listening on port ${process.env.EXPRESS_PORT}`);
});
