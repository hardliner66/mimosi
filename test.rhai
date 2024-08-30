for (sensor, i) in mouse.sensors {
  print("Sensor " + i + ": " + sensor.value);
}

let straight = false;

if straight {
  mouse.right_power = 1.0;
  mouse.left_power = 1.0;
} else {
  mouse.right_power = 0.2;
  mouse.left_power = 0.1;
}
