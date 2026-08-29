import type { CanvasNodeBounds } from './types';

export interface TextOverflowMeasurement {
	currentWidth: number;
	currentHeight: number;
	contentWidth: number;
	contentHeight: number;
}

/**
 * Grow a free-text node just enough to contain the browser's rendered text.
 * Dimensions never shrink: a user-selected width remains the wrapping width,
 * while unusually wide unbreakable content may widen the node as well.
 */
export function getAutoGrowBounds(
	measurement: TextOverflowMeasurement,
	position: Pick<CanvasNodeBounds, 'x' | 'y'>
): CanvasNodeBounds | null {
	const width = Math.ceil(Math.max(measurement.currentWidth, measurement.contentWidth));
	const height = Math.ceil(Math.max(measurement.currentHeight, measurement.contentHeight));

	if (
		width <= Math.ceil(measurement.currentWidth) &&
		height <= Math.ceil(measurement.currentHeight)
	) {
		return null;
	}

	return { ...position, width, height };
}
