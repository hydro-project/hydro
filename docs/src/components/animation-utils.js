import React from 'react';
import KeyGroup from './KeyGroup';
import OperatorBox from './OperatorBox';
import CollectionBox from './CollectionBox';
import Arrow from './Arrow';

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
export const computeArrowPosition = (fromBox, toBox) => {
    // Get bounding boxes (assuming they have getBounds() method or similar)
    const fromBounds = fromBox.getBounds ? fromBox.getBounds() : fromBox;
    const toBounds = toBox.getBounds ? toBox.getBounds() : toBox;

    // Calculate arrow start and end points
    const startX = fromBounds.right;
    const startY = fromBounds.centerY;
    const endX = toBounds.left;
    const endY = toBounds.centerY;

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
    createArrow(id, fromObject, toObject) {
        const arrowPos = computeArrowPosition(fromObject, toObject);
        const arrowElement = (
            <Arrow 
                key={id}
                startX={arrowPos.startX} 
                startY={arrowPos.startY} 
                endX={arrowPos.endX} 
                endY={arrowPos.endY} 
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
        } = options;

        const textElement = (
            <foreignObject
                key={id}
                x={position.x - width / 2}
                y={position.y - height / 2}
                width={width}
                height={height}
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
                    <span id={id}>{text}</span>
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

    // Clear all elements and objects
    clear() {
        this.elements = [];
        this.objects = {};
    }
}
