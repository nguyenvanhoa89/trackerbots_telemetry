## 1. Install dependencies:

```
sudo add-apt-repository ppa:george-edison55/cmake-3.x -y
sudo apt-get update
sudo apt-get install python-argparse git-core wget zip python-empy qtcreator cmake build-essential genromfs -y
sudo apt-get install ant protobuf-compiler libeigen3-dev libopencv-dev openjdk-8-jdk openjdk-8-jre clang-3.5 lldb-3.5 -y
```

## 2. Download simulator source

```
git clone https://github.com/PX4/Firmware.git --depth=1
cd Firmware
git submodule update --init --recursive
```

## 3. Compile and run the simulator

```
make posix_sitl_default jmavsim
```

## 4. Start the Mavlink telemetry host tool

```
./telemetry_host
```


 - Read telemetry data by sending a GET request to `localhost:8000`
 - Write new locations by sending a PUT request to `localhost:8000` with the body:
    ```json
    { "x": ...., "y": ...., "alt": .... }
    ```
