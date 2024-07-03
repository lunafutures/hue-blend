import dotenv from "dotenv";
dotenv.config();

import { NowChange, nowChangeSchema } from "./colorManager";

test('Validate now change with mirek and brightness', () => {
	const input = `{
		"now": "1999-asdf",
		"change_action": {
			"color": {
				"mirek": 500,
				"brightness": 50
			}
		},
		"just_updated": false
	}`;
	const { error, value: nowChange } = nowChangeSchema.validate(JSON.parse(input));
	expect(error).toBeUndefined();

	const expectedNowChange: NowChange = {
		now: "1999-asdf",
		change_action: {
			color: {
				mirek: 500,
				brightness: 50,
			},
		},
		just_updated: false,
	};
	expect(nowChange).toEqual(expectedNowChange);
});

test('Validate now change with no change', () => {
	const input = `{
		"now": "2005-jkl",
		"change_action": "none",
		"just_updated": true
	}`;
	const { error, value: nowChange } = nowChangeSchema.validate(JSON.parse(input));
	expect(error).toBeUndefined();

	const expectedNowChange: NowChange = {
		now: "2005-jkl",
		change_action: "none",
		just_updated: true,
	};
	expect(nowChange).toEqual(expectedNowChange);
});