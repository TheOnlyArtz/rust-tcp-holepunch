# rust-tcp-holepunch

#### Prerequisites
Your rendezvous server must lay in a network which **doesn't** have a NAT!
The peers may or may not lay in the same network

### Getting Started
`server`
(The server runs on port 3000)
Running the rendezvous server should be as simple as running
```
cargo r
```
or (assuming you've compiled)
```
./server.exe
```

### Example
running the server on a VPS **(without a NAT!!!!)**
```
./server.exe
```

`client A on network A`
```
./client.exe
```

`client B on network B`
```
./client.exe
```
##### How it looks in Wireshark
![image](https://user-images.githubusercontent.com/30025874/139489923-10f50ea0-ca83-47c6-80f2-8f187725db22.png)


The peers may now communicate with each other.
you may close the rendezvous server and watch as they speak without the need of port-forwarding!
