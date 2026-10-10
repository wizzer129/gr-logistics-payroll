import { extractLoadFromScreenshot } from '../services/load-extraction-service';
import { HttpError } from '../middleware/auth';

export async function extractLoad(req: Request): Promise<Response> {
	const form = await req.formData();
	const file = form.get('screenshot');

	if (!(file instanceof File)) throw new HttpError(400, 'Missing screenshot file');

	const buffer = await file.arrayBuffer();
	const base64 = Buffer.from(buffer).toString('base64');

	const extracted = await extractLoadFromScreenshot(base64, file.type);
	return Response.json(extracted);
}
