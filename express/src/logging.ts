import winston from "winston";
import moment from "moment-timezone";
import Joi from "joi";

import { EnvValidator } from "./envValidator";

interface ProcessEnv {
	TIMEZONE: string;
}
const envSchema = Joi.object<ProcessEnv>({
	TIMEZONE: Joi.string().required(),
});
const env = new EnvValidator<ProcessEnv>(envSchema);

const myFormat = winston.format.printf(({ level, message, timestamp }) => {
	const localTimestamp = moment(timestamp).tz(env.getProperty('TIMEZONE'));
	return `${localTimestamp} ${level}: ${message}`;
});

export const logger = winston.createLogger({
	level: 'debug',
	format: winston.format.combine(
		winston.format.colorize(),
		winston.format.timestamp(),
		myFormat,
	),
	transports: [
		new winston.transports.Console(),
	],
});