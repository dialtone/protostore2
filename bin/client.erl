#!/usr/bin/env escript
%%! -pa ./bin
%% first build the bear.erl file `erlc bear.erl`
%% ./bin/client.erl 127.0.0.1 8080 10 10 10 10000
%%

-mode(native).

-include_lib("kernel/include/file.hrl").

connect({Host, Port}=Addr) ->
    io:format("Connecting: ~p~n", [Addr]),
    case gen_tcp:connect(Host, Port, [binary, {active, true}]) of
        {ok, S} -> io:format("Connected: ~p~n", [S]), S;
        E -> io:format("Couldn't connect: ~p~n", [E]), throw(could_not_connect)
    end.

main([Host, PortString, NumProcsString, RuntimeString]) ->
    Port = list_to_integer(PortString),
    NumProcs = list_to_integer(NumProcsString),
    Runtime = list_to_integer(RuntimeString),


    DummyData = binary:part(
        base64:encode(
            crypto:strong_rand_bytes(16)), 0, 16),

    io:format("Starting ~p clients~n", [NumProcs]),
    Pids = lists:map(fun (_) ->
                spawn_link(fun () ->
                                Sock = case gen_tcp:connect(Host, Port, [binary, {active, true}]) of
                                            {ok, S} -> S;
                                            _ -> throw(could_not_connect)
                                        end,
                                hammer(Sock, DummyData) end)
        end,
        lists:seq(1, NumProcs)),

    io:format("Running for ~p seconds~n", [Runtime]),
    [Pid ! hammertime || Pid <- Pids],
    timer:sleep(Runtime * 1000),

    [Pid ! {halt, self()} || Pid <- Pids],
    io:format("============================~n"),

    done = recv_all(Pids).



hammer(Sock, DummyData) ->
    ReqId = 0,
    receive hammertime -> ok end,
    hammer(Sock, DummyData, ReqId).

hammer(Sock, DummyData, ReqId) ->
    receive
        {halt, Pid} ->
            block_recv(Sock),
            ok = gen_tcp:close(Sock),
            Pid ! {self(), results}
    after 0 ->
        Body = <<(1+16+4):32/unsigned-integer, "R", DummyData/binary, ReqId:32/unsigned-integer>>,
        io:format("sending reqid: ~p~n", [ReqId]),
        io:format("sending body of len: ~p~n", [size(Body)]),
        io:format("sending body: ~p~n", [Body]),
        ok = gen_tcp:send(Sock, Body),
        block_recv(Sock),
        hammer(Sock, DummyData, ReqId+1)
    end.


block_recv(Sock) ->
    block_recv(Sock, <<>>).

block_recv(Sock, Partial) ->
    receive
        {tcp, Sock, Data} ->
            parse(Sock, <<Partial/binary, Data/binary>>)
    end.

parse(Sock, Data) ->
    case Data of
        <<>> ->
            undefined;

        <<_:32/unsigned-integer,  0:16/unsigned-integer, _/binary>> ->
            io:format("bad data: ~p~n", [Data]),
            throw(bad_response);

        <<ReqId:32/unsigned-integer, Len:16/unsigned-integer, Body/binary>> ->
            case byte_size(Body) >= Len of
                true ->
                    <<_ResponseBody:Len/binary, Rest/binary>> = Body,
                    parse(Sock, Rest);
                false ->
                    block_recv(Sock, Data)
            end;

        _ ->
            block_recv(Sock, Data)
    end.



recv_all([]) ->
    done;
recv_all([Pid|Pids]) ->
    receive
        {Pid, results} ->
            recv_all(Pids)
    after 10000 ->
            io:format("Could not receive results from ~p~n", [Pid]),
            error
    end.
