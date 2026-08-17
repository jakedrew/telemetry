import './style.css';
import {Map, View} from 'ol';
import TileLayer from 'ol/layer/Tile';
import OSM from 'ol/source/OSM';
import {fromLonLat} from 'ol/proj';

import VectorLayer from 'ol/layer/Vector';
import VectorSource from 'ol/source/Vector';
import Feature from 'ol/Feature';
import {fromExtent} from 'ol/geom/Polygon';

import Point from 'ol/geom/Point';
import {Style, Stroke, Fill, Text, Icon, Circle as CircleStyle} from 'ol/style';

import LineString from 'ol/geom/LineString';

import Overlay from 'ol/Overlay';

// isle of wight
const center_point = fromLonLat([-1.297848, 50.676592]);

const map = new Map({
    target: 'map',
    layers: [
        new TileLayer({
        source: new OSM()
        })
    ],
    view: new View({
        center: center_point, 
        zoom: 11 
    })
});

// border of the are of interest 
const halfSize = 10000; 

const square = fromExtent([
    center_point[0] - halfSize,
    center_point[1] - halfSize,
    center_point[0] + halfSize,
    center_point[1] + halfSize,
]);

const vectorLayer = new VectorLayer({
    source: new VectorSource({
        features: [new Feature(square)],
    }),
    style: new Style({
        stroke: new Stroke({
        color: 'red',
        width: 2,
        lineDash: [4, 8],
        }),
    }),
});

map.addLayer(vectorLayer)

function moveMarker(id, lon, lat) {
    vehicles[id].marker.getGeometry().setCoordinates(fromLonLat([lon, lat]));

    const coord = fromLonLat([lon, lat]);

    vehicles[id].marker.getGeometry().setCoordinates(coord);

    const coords = vehicles[id].trail.getGeometry().getCoordinates();
    if (coords.length > 0) {
        const prev = coords[coords.length - 1];
        vehicles[id].marker.getStyle().getImage().setRotation(
            Math.atan2(coord[0] - prev[0], coord[1] - prev[1])
        );
    }
    coords.push(coord);

    if (coords.length > 25) coords.shift();
    vehicles[id].trail.getGeometry().setCoordinates(coords);
}

var es = new EventSource('http://localhost:3000/stream');

var vehicles = {};

var svg = '<svg width="120" height="120" viewBox="-10 -10 20 20" version="1.1" xmlns="http://www.w3.org/2000/svg">'
    + '<rect x="-9" y="-9" width="18" height="18" fill="#fff" stroke="#999" stroke-width="0.6"/>'
    + '<path d="M0,-6 L4,5 L0,2.5 L-4,5 Z" fill="#000000"/>'
    + '</svg>';

es.onmessage = function (e) {
    var m = JSON.parse(e.data);
    vehicles[m.id] = Object.assign(vehicles[m.id] || {}, m);

    if (vehicles[m.id].colour == null) {
        vehicles[m.id].colour = 'hsl(' + Math.floor(Math.random() * 360) + ', 70%, 45%)';
    }

    if (vehicles[m.id].marker == null) {
        vehicles[m.id].marker = new Feature(new Point(fromLonLat([0, 0])));
        vehicles[m.id].marker.set('id', m.id); 
        vehicles[m.id].marker.setStyle(new Style({
            image: new Icon({
              opacity: 1,
              src: 'data:image/svg+xml;utf8,' + encodeURIComponent(svg.replace("#000000", vehicles[m.id].colour)),
              scale: 0.18
            }),
            text: new Text({
                text: m.id,
                font: '12px monospace',
                offsetX: 22,
                textAlign: 'left',
                padding: [3, 5, 2, 5],
                fill: new Fill({color: '#000'}),
                backgroundFill: new Fill({color: '#fff'}),
                backgroundStroke: new Stroke({color: '#999', width: 1}),
            })
          }));
        // vehicles[m.id].marker.setStyle(new Style({
        //     image: new CircleStyle({
        //     radius: 7,
        //     fill: new Fill({color: vehicles[m.id].colour}),
        //     stroke: new Stroke({color: 'white', width: 2}),
        //     }),
        // }));

        map.addLayer(new VectorLayer({
            source: new VectorSource({features: [vehicles[m.id].marker]}),
            zIndex: 2
        }));
    }
    if (vehicles[m.id].trail == null) {
        vehicles[m.id].trail = new Feature(new LineString([]));
        vehicles[m.id].trail.setStyle(new Style({
            stroke: new Stroke({color: vehicles[m.id].colour, width: 2}),
        }));
        
        map.addLayer(new VectorLayer({
            source: new VectorSource({features: [vehicles[m.id].trail]}),
            zIndex: 1
        }));
    }
    
    moveMarker(m.id, m.longitude, m.latitude);
};
// document.getElementById('output').textContent = e.data;

es.onerror = function () {
    console.error('onerror hit');
};