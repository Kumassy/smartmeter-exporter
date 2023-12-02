# smartmeter-exporter
スマートメーターから瞬間消費電力を取得する Prometheus Exporter の Rust 実装

![metrics.png](docs/metrics.png)


## 背景
家庭用電力に使用量はスマートメーターによって自動的に測定されている。
スマートメーターが計測している情報は、Bルートを使うことで一般人も確認可能である。


> 電力メーター情報発信サービス（Bルートサービス）  
> https://www.tepco.co.jp/pg/consignment/liberalization/smartmeter-broute.html

## 必要なもの
以下の物品を用意した

- 必須
    - B ルートの ID, Password
        - https://www.tepco.co.jp/pg/consignment/liberalization/smartmeter-broute.html から入手する
    - Raspberry Pi3 Model B
    - BP35A1 （Wi-SUN モジュール）
        - https://www.zaikostore.com/zaikostore/stockDetail?productIdOfHitotsukara=pr6714723
- あると便利なもの
    - BP35A7A （BP35A1用変換基板）
        - https://www.zaikostore.com/zaikostore/stockDetail?productIdOfHitotsukara=pr8499490
    - BP35A7-accessories （BP35A1用アクセサリ）
        - https://www.zaikostore.com/zaikostore/stockDetail?productIdOfHitotsukara=pr7853764

![parts](docs/parts.jpg)

## Raspberry Pi を設定する
適当な方法で Raspberry Pi OS (32bit, kernel version: 5.15, debian version: 11.6) を microSD カードにインストールし、 Raspberry Pi を起動した

```
pi@raspberrypi:~/smartmeter-exporter $ cat /etc/os-release
PRETTY_NAME="Raspbian GNU/Linux 11 (bullseye)"
NAME="Raspbian GNU/Linux"
VERSION_ID="11"
VERSION="11 (bullseye)"
VERSION_CODENAME=bullseye
ID=raspbian
ID_LIKE=debian
HOME_URL="http://www.raspbian.org/"
SUPPORT_URL="http://www.raspbian.org/RaspbianForums"
BUG_REPORT_URL="http://www.raspbian.org/RaspbianBugs"
pi@raspberrypi:~/smartmeter-exporter $ uname -ar
Linux raspberrypi 5.15.84-v7+ #1613 SMP Thu Jan 5 11:59:48 GMT 2023 armv7l GNU/Linux
pi@raspberrypi:~/smartmeter-exporter $ cat /etc/debian_version
11.6
```

以下の記事を参考に、シリアル通信を有効化した

> Raspberry Pi GPIOを使用したシリアル通信  
> https://www.ingenious.jp/articles/howto/raspberry-pi-howto/gpio-uart/

```
pi@raspberrypi:~ $ sudo systemctl disable hciuart
Removed /etc/systemd/system/dev-serial1.device.wants/hciuart.service.

pi@raspberrypi:~ $ sudo raspi-config

# 3 Interface Options
# I6 Serial Port
# Would you like a login shell to be accessible over serial -> No
# Would you like the serial port hardware to be enabled? -> Yes
```

![enable serial port](docs/raspberry_enable_serial.png)

```
pi@raspberrypi:~ $ sudo vi /boot/config.txt
pi@raspberrypi:~ $ sudo tail /boot/config.txt

[all]

[pi4]
# Run as fast as firmware / board allows
arm_boost=1

[all]
enable_uart=1
dtoverlay=disable-bt # <-- 追記

pi@raspberrypi:~ $ sudo reboot now
```


```
pi@raspberrypi:~ $ ls -l /dev/  | grep serial
lrwxrwxrwx 1 root root           7 Apr 15 09:19 serial0 -> ttyAMA0
lrwxrwxrwx 1 root root           5 Apr 15 09:19 serial1 -> ttyS0
```

## Wi-SUN モジュールを Raspberry Pi に接続する

次のように配線する

| Raspberry Pi | BP35A7A |
|----------|----------|
| 1: 3v3 Power   | CN1.4: VCC  |
| 6: Ground   | CN2.9: CND  |
| 8: GPIO 14 (UART TX)   | CN2.4: RXD |
| 10: GPIO 15 (UART RX)   | CN2.5: TXD |

![wiring](docs/wiring.jpg)


ピンアサインの参考資料:

> Raspberry Pi GPIO Pinout  
> https://pinout.xyz/

> BP35A7A 評価ボード  
> https://fscdn.rohm.com/jp/products/databook/applinote/module/wireless/bp35a7a_evaluationboard_ug-j.pdf

## smartmeter-exporter のビルドと実行
Raspberry Pi 上でビルドするのは時間がかかるので、 [cross](https://github.com/cross-rs/cross) を使ってビルドした


ビルド環境
```
% sw_vers
ProductName:            macOS
ProductVersion:         13.6
BuildVersion:           22G120
% docker version
Client:
 Cloud integration: v1.0.20
 Version:           20.10.10
 API version:       1.41
 Go version:        go1.16.9
 Git commit:        b485636
 Built:             Mon Oct 25 07:43:15 2021
 OS/Arch:           darwin/arm64
 Context:           default
 Experimental:      true

Server: Docker Engine - Community
 Engine:
  Version:          20.10.10
  API version:      1.41 (minimum version 1.12)
  Go version:       go1.16.9
  Git commit:       e2f740d
  Built:            Mon Oct 25 07:41:10 2021
  OS/Arch:          linux/arm64
  Experimental:     false
 containerd:
  Version:          1.4.11
  GitCommit:        5b46e404f6b9f661a205e28d59c982d3634148f8
 runc:
  Version:          1.0.2
  GitCommit:        v1.0.2-0-g52b36a2
 docker-init:
  Version:          0.19.0
  GitCommit:        de40ad0
```

ビルド
```
% cd /path/to/smartmeter-exporter
% vi .env
% cat .env
export B_ID=0000XXXXXX
export B_PW=XXXXX

% cross build --target armv7-unknown-linux-gnueabihf
```

適当な方法で `./target/armv7-unknown-linux-gnueabihf/debug/smartmeter-exporter` を `/home/pi/smartmeter-exporter/smartmeter-exporter` にコピー


実行
```
RUST_LOG=debug /home/pi/smartmeter-exporter/smartmeter-exporter
```


## Grafana Cloud に継続的に測定結果を送信する

smartmeter-exporter を systemd を使って管理することにした

```
pi@raspberrypi:~/smartmeter-exporter$ sudo vim /etc/systemd/system/smartmeter-exporter.service
pi@raspberrypi:~/smartmeter-exporter$ cat /etc/systemd/system/smartmeter-exporter.service
[Unit]
Description=Smartmeter Exporter

[Service]
ExecStart=/home/pi/smartmeter-exporter/smartmeter-exporter
Environment="RUST_LOG_DESTINATION=file"
Environment="RUST_LOG=info"
Restart=always

[Install]
WantedBy=multi-user.target

pi@raspberrypi:~/smartmeter-exporter $ sudo mkdir /var/log/smartmeter-exporter
pi@raspberrypi:~/smartmeter-exporter $ sudo mchmod g+w /var/log/smartmeter-exporter
pi@raspberrypi:~/smartmeter-exporter $ sudo mchmod g+s /var/log/smartmeter-exporter
pi@raspberrypi:~/smartmeter-exporter $ cat /etc/logrotate.d/smartmeter-exporter
/var/log/smartmeter-exporter/smartmeter-exporter.log {
    su root adm
    rotate 14
    daily
    compress
    missingok
    delaycompress
    copytruncate
}
pi@raspberrypi:~/smartmeter-exporter$ sudo systemctl daemon-reload
pi@raspberrypi:~/smartmeter-exporter$ sudo systemctl enable smartmeter-exporter --now
```

以下のページに従って、Grafana Agent をインストールした


> Install Grafana Agent in static mode on Linux  
> https://grafana.com/docs/agent/latest/static/set-up/install/install-agent-linux/


```
pi@raspberrypi:~/smartmeter-exporter$ mkdir -p /etc/apt/keyrings/
pi@raspberrypi:~/smartmeter-exporter$ wget -q -O - https://apt.grafana.com/gpg.key | gpg --dearmor | sudo tee /etc/apt/keyrings/grafana.gpg
pi@raspberrypi:~/smartmeter-exporter$ echo "deb [signed-by=/etc/apt/keyrings/grafana.gpg] https://apt.grafana.com stable main" | sudo tee /etc/apt/sources.list.d/grafana.list
pi@raspberrypi:~/smartmeter-exporter$ sudo apt-get update
pi@raspberrypi:~/smartmeter-exporter$ sudo apt-get install grafana-agent

pi@raspberrypi:~/smartmeter-exporter$ sudo vim /usr/local/etc/grafana-agent/agent-config.yaml
pi@raspberrypi:~/smartmeter-exporter$ cat /usr/local/etc/grafana-agent/agent-config.yaml

metrics:
  global:
    scrape_interval: 60s
  configs:
  - name: smartmeter
    scrape_configs:
      - job_name: smartmeter-metrics
        static_configs:
        - targets: ['localhost:9186']
    remote_write:
      - url: https://prometheus-us-central1.grafana.net/api/prom/push
        basic_auth:
          username: 111111
          password: supersecret

logs:
  configs:
  - name: smartmeter
    positions:
      filename: /tmp/smartmeter_logs_positions.yml
    scrape_configs:
      - job_name: smartmeter-logs
        static_configs:
        - targets: [localhost]
          labels:
            job: smartmeter-logs
            __path__: /var/log/smartmeter-exporter/smartmeter-exporter.log
    clients:
    -  url: https://11111:supersecret@logs-prodX.grafana.net/api/prom/push

pi@raspberrypi:~/smartmeter-exporter $ cat /etc/systemd/system/grafana-agent.service
[Unit]
Description=Grafana Agent

[Service]
User=root
ExecStart=/usr/bin/grafana-agent --config.file=/usr/local/etc/grafana-agent/agent-config.yaml
Restart=always

[Install]
WantedBy=multi-user.target
pi@raspberrypi:~/smartmeter-exporter$ sudo systemctl enable grafana-agent --now
```

## Grafana Cloud でダッシュボードを作成
Grafana Cloud では `instantaneous_energy` というメトリクスを参照することで瞬間消費電力を確認できる

![grafana metrics](docs/grafana_metrics.png)

smartmeter-exporter のログを組み合わせて以下のようなダッシュボードを作成した


![grafana dashboard](docs/grafana_dashboard.png)

## ライセンス
MIT