import React from 'react';
import KeyGroup from './KeyGroup';
import OperatorBox from './OperatorBox';
import CollectionBox from './CollectionBox';
import Arrow from './Arrow';
import SlicedRegion from './SlicedRegion';

// Point class for representing 2D coordinates
export class Point {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }

    // Static factory method for creating points
    static create(x, y) {
        return new Point(x, y);
    }
}

// Utility function to compute center positions for elements in a content box using space-around distribution
export const computeElementPositions = (contentTop, contentBottom, elementHeight, numElements) => {
    const contentHeight = contentBottom - contentTop;
    const totalElementsHeight = numElements * elementHeight;
    const totalSpaceHeight = contentHeight - totalElementsHeight;

    // Space-around: equal space above, below, and between elements
    // This creates (numElements + 1) equal spaces
    const spaceUnit = totalSpaceHeight / (numElements + 1);

    const positions = [];
    for (let i = 0; i < numElements; i++) {
        // Position each element center: top + space + (element spaces so far) + (elements so far) + half current element
        const y = contentTop + spaceUnit + (i * spaceUnit) + (i * elementHeight) + (elementHeight / 2);
        positions.push(y);
    }
    return positions;
};

// Utility function to compute arrow positions between two bounding boxes
// Options: startSide and endSide can be 'left', 'right', 'top', 'bottom', or 'auto' (default)
export const computeArrowPosition = (fromBox, toBox, options = {}) => {
    const { startSide = 'auto', endSide = 'auto' } = options;
    
    // Get bounding boxes (assuming they have getBounds() method or similar)
    const fromBounds = fromBox.getBounds ? fromBox.getBounds() : fromBox;
    const toBounds = toBox.getBounds ? toBox.getBounds() : toBox;

    // Determine arrow direction based on relative positions
    const dx = toBounds.centerX - fromBounds.centerX;
    const dy = toBounds.centerY - fromBounds.centerY;

    let startX, startY, endX, endY;

    // Compute start position
    if (startSide === 'auto') {
        if (Math.abs(dx) > Math.abs(dy)) {
            startX = dx > 0 ? fromBounds.right : fromBounds.left;
            startY = fromBounds.centerY;
        } else {
            startX = fromBounds.centerX;
            startY = dy > 0 ? fromBounds.bottom : fromBounds.top;
        }
    } else {
        switch (startSide) {
            case 'left': startX = fromBounds.left; startY = fromBounds.centerY; break;
            case 'right': startX = fromBounds.right; startY = fromBounds.centerY; break;
            case 'top': startX = fromBounds.centerX; startY = fromBounds.top; break;
            case 'bottom': startX = fromBounds.centerX; startY = fromBounds.bottom; break;
        }
    }

    // Compute end position
    if (endSide === 'auto') {
        if (Math.abs(dx) > Math.abs(dy)) {
            endX = dx > 0 ? toBounds.left : toBounds.right;
            endY = toBounds.centerY;
        } else {
            endX = toBounds.centerX;
            endY = dy > 0 ? toBounds.top : toBounds.bottom;
        }
    } else {
        switch (endSide) {
            case 'left': endX = toBounds.left; endY = toBounds.centerY; break;
            case 'right': endX = toBounds.right; endY = toBounds.centerY; break;
            case 'top': endX = toBounds.centerX; endY = toBounds.top; break;
            case 'bottom': endX = toBounds.centerX; endY = toBounds.bottom; break;
        }
    }

    return { startX, startY, endX, endY };
};

// Object-oriented KeyGroup class
export class KeyGroupClass {
    constructor(position, width, height, keyName, color, backgroundColor = 'white') {
        this.position = position; // Point object for center coordinates
        this.width = width;
        this.height = height;
        this.keyName = keyName;
        this.color = color;
        this.backgroundColor = backgroundColor;
        this.headerHeight = 18; // Fixed header height from KeyGroup component
    }

    // Get the bounding box of the content area (excluding header)
    getContentBounds() {
        const left = this.position.x - this.width / 2;
        const right = this.position.x + this.width / 2;
        const top = this.position.y - this.height / 2 + this.headerHeight;
        const bottom = this.position.y + this.height / 2;

        return {
            left,
            right,
            top,
            bottom,
            width: this.width,
            height: this.height - this.headerHeight,
            centerX: this.position.x,
            centerY: top + (bottom - top) / 2
        };
    }

    // Get the center coordinate of the content area
    getContentCenter() {
        const bounds = this.getContentBounds();
        return new Point(bounds.centerX, bounds.centerY);
    }

    // Return the React element
    toReactElement() {
        return (
            <KeyGroup
                x={this.position.x}
                y={this.position.y}
                width={this.width}
                height={this.height}
                keyName={this.keyName}
                color={this.color}
                backgroundColor={this.backgroundColor}
            />
        );
    }
}

// Object-oriented CollectionBox class
export class CollectionBoxClass {
    constructor(position, width, height, headerHeight, title, headerColor = '#777', borderColor = '#777', backgroundColor = 'white') {
        this.position = position; // Point object for center coordinates
        this.width = width;
        this.height = height;
        this.headerHeight = headerHeight;
        this.title = title;
        this.headerColor = headerColor;
        this.borderColor = borderColor;
        this.backgroundColor = backgroundColor;
    }

    // Get the bounding box of the content area (excluding header)
    getContentBounds() {
        const left = this.position.x - this.width / 2;
        const right = this.position.x + this.width / 2;
        const top = this.position.y - this.height / 2 + this.headerHeight;
        const bottom = this.position.y + this.height / 2;

        return {
            left,
            right,
            top,
            bottom,
            width: this.width,
            height: this.height - this.headerHeight,
            centerX: this.position.x,
            centerY: top + (bottom - top) / 2
        };
    }

    // Get the center coordinate of the content area
    getContentCenter() {
        const bounds = this.getContentBounds();
        return new Point(bounds.centerX, bounds.centerY);
    }

    // Get the overall bounding box (including header)
    getBounds() {
        const left = this.position.x - this.width / 2;
        const right = this.position.x + this.width / 2;
        const top = this.position.y - this.height / 2;
        const bottom = this.position.y + this.height / 2;

        return {
            left,
            right,
            top,
            bottom,
            width: this.width,
            height: this.height,
            centerX: this.position.x,
            centerY: this.position.y
        };
    }

    // Compute positions for child elements within the content area
    verticalChildPositions(elementHeight, numElements) {
        const bounds = this.getContentBounds();
        const yPositions = computeElementPositions(bounds.top, bounds.bottom, elementHeight, numElements);
        // Return Point objects that are horizontally centered
        return yPositions.map(y => new Point(bounds.centerX, y));
    }

    // Return the React element
    toReactElement() {
        return (
            <CollectionBox
                key={this.title}
                x={this.position.x}
                y={this.position.y}
                width={this.width}
                height={this.height}
                headerHeight={this.headerHeight}
                title={this.title}
                headerColor={this.headerColor}
                borderColor={this.borderColor}
                backgroundColor={this.backgroundColor}
            />
        );
    }
}

// Object-oriented OperatorBox class
export class OperatorBoxClass {
    constructor(position, width, height, text, backgroundColor = '#f0f0f0', borderColor = '#777', textColor = '#333') {
        this.position = position; // Point object for center coordinates
        this.width = width;
        this.height = height;
        this.text = text;
        this.backgroundColor = backgroundColor;
        this.borderColor = borderColor;
        this.textColor = textColor;
    }

    // Get the bounding box
    getBounds() {
        const left = this.position.x - this.width / 2;
        const right = this.position.x + this.width / 2;
        const top = this.position.y - this.height / 2;
        const bottom = this.position.y + this.height / 2;

        return {
            left,
            right,
            top,
            bottom,
            width: this.width,
            height: this.height,
            centerX: this.position.x,
            centerY: this.position.y
        };
    }

    // Return the React element
    toReactElement() {
        return (
            <OperatorBox
                key={this.text}
                x={this.position.x}
                y={this.position.y}
                width={this.width}
                height={this.height}
                text={this.text}
                backgroundColor={this.backgroundColor}
                borderColor={this.borderColor}
                textColor={this.textColor}
            />
        );
    }
}

// World class that manages object creation and rendering
export class World {
    constructor(width, height) {
        this.width = width;
        this.height = height;
        this.elements = [];
        this.objects = {};
    }

    // Get the center coordinates of the world
    getCenter() {
        return new Point(this.width / 2, this.height / 2);
    }

    // Create a collection box and add to render list
    createCollectionBox(id, position, width, height, headerHeight, title, headerColor = '#777', borderColor = '#777', backgroundColor = "transparent") {
        const box = new CollectionBoxClass(position, width, height, headerHeight, title, headerColor, borderColor, backgroundColor);
        this.elements.push(box.toReactElement());
        this.objects[id] = box;
        return box;
    }

    // Create an operator box and add to render list
    createOperatorBox(id, position, width, height, text, backgroundColor = 'transparent', borderColor = '#777', textColor = 'inherit') {
        const box = new OperatorBoxClass(position, width, height, text, backgroundColor, borderColor, textColor);
        this.elements.push(box.toReactElement());
        this.objects[id] = box;
        return box;
    }

    // Create a key group and add to render list
    createKeyGroup(id, position, width, height, keyName, color, backgroundColor = "transparent") {
        const group = new KeyGroupClass(position, width, height, keyName, color, backgroundColor);
        this.elements.push(group.toReactElement());
        this.objects[id] = group;
        return group;
    }

    // Create an arrow between two objects and add to render list
    createArrow(id, fromObject, toObject, options = {}) {
        const { dashed = false, strokeDasharray = '4,3', opacity = 1, startSide = 'auto', endSide = 'auto' } = options;
        const arrowPos = computeArrowPosition(fromObject, toObject, { startSide, endSide });
        const arrowElement = (
            <Arrow 
                key={id}
                id={id}
                startX={arrowPos.startX} 
                startY={arrowPos.startY} 
                endX={arrowPos.endX} 
                endY={arrowPos.endY}
                dashed={dashed}
                strokeDasharray={strokeDasharray}
                opacity={opacity}
            />
        );
        this.elements.push(arrowElement);
        return arrowPos;
    }

    // Add a custom element to the render list
    addElement(element) {
        this.elements.push(element);
    }

    // Add a text element centered at the given coordinates
    addText(id, position, text, options = {}) {
        const {
            width = 40,
            height = 20,
            fontSize = '10px',
            fontWeight = 'bold',
            fontFamily = 'monospace',
            opacity = 1,
        } = options;

        const textElement = (
            <foreignObject
                key={id}
                id={id}
                x={position.x - width / 2}
                y={position.y - height / 2}
                width={width}
                height={height}
                opacity={opacity}
            >
                <div
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        height: '100%',
                        fontSize,
                        fontWeight,
                        fontFamily,
                    }}
                >
                    <span>{text}</span>
                </div>
            </foreignObject>
        );
        
        this.elements.push(textElement);
        return textElement;
    }

    // Get object by ID
    getObject(id) {
        return this.objects[id];
    }

    // Get all elements for rendering
    getElements() {
        return this.elements;
    }

    // Create a sliced region (dotted rectangle) and add to render list
    createSlicedRegion(id, position, width, height, label = 'sliced!', options = {}) {
        const {
            strokeColor = '#888',
            strokeWidth = 2,
            strokeDasharray = '6,4',
            labelColor = '#666',
            labelFontSize = 10,
            rx = 8,
            opacity = 1,
            labelPosition = 'left'
        } = options;

        const region = {
            position,
            width,
            height,
            getBounds: () => ({
                left: position.x - width / 2,
                right: position.x + width / 2,
                top: position.y - height / 2,
                bottom: position.y + height / 2,
                width,
                height,
                centerX: position.x,
                centerY: position.y
            }),
            getCenter: () => new Point(position.x, position.y)
        };

        const element = (
            <SlicedRegion
                key={id}
                id={id}
                x={position.x}
                y={position.y}
                width={width}
                height={height}
                label={label}
                strokeColor={strokeColor}
                strokeWidth={strokeWidth}
                strokeDasharray={strokeDasharray}
                labelColor={labelColor}
                labelFontSize={labelFontSize}
                rx={rx}
                opacity={opacity}
                labelPosition={labelPosition}
            />
        );

        this.elements.push(element);
        this.objects[id] = region;
        return region;
    }

    // Clear all elements and objects
    clear() {
        this.elements = [];
        this.objects = {};
    }
}
