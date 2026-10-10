import Anthropic from '@anthropic-ai/sdk';
import { z } from 'zod';
import { env } from '../utils/env';

const anthropic = new Anthropic({ apiKey: env.anthropicApiKey });

const extractedLoadSchema = z.object({
	date: z.string(),
	storeName: z.string(),
	payment: z.number(),
	extraPay: z.number().default(0),
});

type SupportedImageMediaType = 'image/jpeg' | 'image/png' | 'image/gif' | 'image/webp';

function assertSupportedImageMediaType(mediaType: string): SupportedImageMediaType {
	const supported: SupportedImageMediaType[] = [
		'image/jpeg',
		'image/png',
		'image/gif',
		'image/webp',
	];

	if (!supported.includes(mediaType as SupportedImageMediaType)) {
		throw new Error(`Unsupported screenshot type: ${mediaType}`);
	}

	return mediaType as SupportedImageMediaType;
}

export async function extractLoadFromScreenshot(imageBase64: string, mediaType: string) {
	const message = await anthropic.messages.create({
		model: 'claude-opus-5',
		max_tokens: 1024,
		messages: [
			{
				role: 'user',
				content: [
					{
						type: 'image',
						source: {
							type: 'base64',
							media_type: assertSupportedImageMediaType(mediaType),
							data: imageBase64,
						},
					},
					{
						type: 'text',
						text: 'Extract the load date, store name/number, payment, and extra pay from this screenshot. Respond with ONLY a JSON object: {"date": "YYYY-MM-DD", "storeName": "...", "payment": 0, "extraPay": 0}.',
					},
				],
			},
		],
	});

	const textBlock = message.content.find((block) => block.type === 'text');
	if (!textBlock || textBlock.type !== 'text')
		throw new Error('No text response from extraction model');

	const parsed = JSON.parse(textBlock.text);
	return extractedLoadSchema.parse(parsed);
}
