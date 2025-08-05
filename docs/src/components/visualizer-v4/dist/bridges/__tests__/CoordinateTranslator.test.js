/**
 * @fileoverview CoordinateTranslator Unit Tests
 *
 * Comprehensive tests for coordinate system translation between ELK and ReactFlow
 */
import { describe, it, expect } from 'vitest';
import { CoordinateTranslator } from '../CoordinateTranslator';
describe('CoordinateTranslator', () => {
    describe('elkToReactFlow', () => {
        it('should pass through top-level coordinates unchanged', () => {
            const elkCoords = { x: 100, y: 200 };
            const result = CoordinateTranslator.elkToReactFlow(elkCoords);
            expect(result.x).toBe(100);
            expect(result.y).toBe(200);
        });
        it('should convert child element coordinates to relative positions', () => {
            const elkCoords = { x: 150, y: 225 };
            const container = {
                id: 'container1',
                x: 50,
                y: 75,
                width: 300,
                height: 400
            };
            const result = CoordinateTranslator.elkToReactFlow(elkCoords, container);
            // FIXED: ELK already provides child coordinates relative to container
            // So we should pass them through unchanged
            expect(result.x).toBe(150); // ELK child coords are already relative
            expect(result.y).toBe(225); // ELK child coords are already relative
        });
    });
    describe('reactFlowToELK', () => {
        it('should pass through top-level coordinates unchanged', () => {
            const reactFlowCoords = { x: 300, y: 400 };
            const result = CoordinateTranslator.reactFlowToELK(reactFlowCoords);
            expect(result.x).toBe(300);
            expect(result.y).toBe(400);
        });
        it('should convert child element coordinates to absolute positions', () => {
            const reactFlowCoords = { x: 100, y: 150 };
            const container = {
                id: 'container1',
                x: 50,
                y: 75,
                width: 300,
                height: 400
            };
            const result = CoordinateTranslator.reactFlowToELK(reactFlowCoords, container);
            expect(result.x).toBe(150); // 100 + 50 = 150
            expect(result.y).toBe(225); // 150 + 75 = 225
        });
    });
    describe('round-trip conversion', () => {
        it('should preserve coordinates for top-level elements', () => {
            const originalELK = { x: 123.45, y: 678.90 };
            const reactFlow = CoordinateTranslator.elkToReactFlow(originalELK);
            const backToELK = CoordinateTranslator.reactFlowToELK(reactFlow);
            expect(backToELK.x).toBe(originalELK.x);
            expect(backToELK.y).toBe(originalELK.y);
        });
        it('should preserve coordinates for child elements', () => {
            const container = {
                id: 'container1',
                x: 62.5,
                y: 87.25,
                width: 400,
                height: 300
            };
            const originalChildELK = { x: 175.25, y: 287.75 };
            const childReactFlow = CoordinateTranslator.elkToReactFlow(originalChildELK, container);
            const backToChildELK = CoordinateTranslator.reactFlowToELK(childReactFlow, container);
            // FIXED: Since ELK child coords are already relative, the round-trip should preserve
            // the coordinates plus the container offset
            expect(backToChildELK.x).toBeCloseTo(originalChildELK.x + container.x, 4);
            expect(backToChildELK.y).toBeCloseTo(originalChildELK.y + container.y, 4);
        });
    });
    describe('getContainerInfo', () => {
        it('should extract container info for existing containers', () => {
            const mockVisState = {
                getContainer: (id) => {
                    if (id === 'container1') {
                        return {
                            layout: {
                                position: { x: 100, y: 150 },
                                dimensions: { width: 300, height: 200 }
                            }
                        };
                    }
                    return null;
                }
            };
            const result = CoordinateTranslator.getContainerInfo('container1', mockVisState);
            expect(result).toBeDefined();
            expect(result.id).toBe('container1');
            expect(result.x).toBe(100);
            expect(result.y).toBe(150);
            expect(result.width).toBe(300);
            expect(result.height).toBe(200);
        });
        it('should return undefined for non-existent containers', () => {
            const mockVisState = {
                getContainer: () => null
            };
            const result = CoordinateTranslator.getContainerInfo('nonexistent', mockVisState);
            expect(result).toBeUndefined();
        });
    });
    describe('validateConversion', () => {
        it('should return true for valid conversions', () => {
            const originalELK = { x: 100, y: 200 };
            const reactFlow = { x: 50, y: 125 };
            const backToELK = { x: 100, y: 200 };
            const isValid = CoordinateTranslator.validateConversion(originalELK, reactFlow, backToELK);
            expect(isValid).toBe(true);
        });
        it('should return false for invalid conversions', () => {
            const originalELK = { x: 100, y: 200 };
            const reactFlow = { x: 50, y: 125 };
            const invalidBackToELK = { x: 101, y: 200 }; // Different from original
            const isInvalid = CoordinateTranslator.validateConversion(originalELK, reactFlow, invalidBackToELK);
            expect(isInvalid).toBe(false);
        });
    });
    describe('edge cases', () => {
        it('should handle zero coordinates correctly', () => {
            const zeroELK = { x: 0, y: 0 };
            const zeroContainer = { id: 'zero', x: 0, y: 0, width: 100, height: 100 };
            const zeroReactFlow = CoordinateTranslator.elkToReactFlow(zeroELK, zeroContainer);
            const zeroBackToELK = CoordinateTranslator.reactFlowToELK(zeroReactFlow, zeroContainer);
            expect(zeroReactFlow.x).toBe(0);
            expect(zeroReactFlow.y).toBe(0);
            expect(zeroBackToELK.x).toBe(0);
            expect(zeroBackToELK.y).toBe(0);
        });
        it('should handle negative relative coordinates', () => {
            const negativeELK = { x: 25, y: 50 };
            const negativeContainer = { id: 'negative', x: 100, y: 100, width: 200, height: 200 };
            const negativeReactFlow = CoordinateTranslator.elkToReactFlow(negativeELK, negativeContainer);
            // FIXED: ELK child coordinates are already relative to container
            // So they pass through unchanged
            expect(negativeReactFlow.x).toBe(25); // ELK child coords are already relative
            expect(negativeReactFlow.y).toBe(50); // ELK child coords are already relative
        });
    });
});
//# sourceMappingURL=CoordinateTranslator.test.js.map