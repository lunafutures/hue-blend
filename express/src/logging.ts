import winston from "winston";

const myFormat = winston.format.printf(({ level, message, timestamp }) => {
	return `${timestamp} ${level}: ${message}`;
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