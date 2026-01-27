using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Threading.Channels;
using GameService;
using Grpc.Core;
using Grpc.Core.Interceptors;
using Grpc.Net.Client;

namespace MiniGameNetworkBot;

public sealed class GameNetworkHandler : IAsyncDisposable
{
    
    public readonly string ClientId;
    
    private volatile bool _disposed;
    
    private readonly GrpcChannel _grpcChannel;
    private readonly IClientStreamWriter<ClientMessage> _streamWriter;
    private readonly IAsyncStreamReader<ServerMessage> _streamReader;
    
    private readonly Task _readTask;
    private readonly Task _writeTask;
    
    private readonly ChannelReader<ClientMessage> _messageQueueReader;
    private readonly ChannelWriter<ClientMessage> _messageQueueWriter;
    
    private readonly CancellationTokenSource _cts = new();
    
    private readonly ConcurrentDictionary<Guid, WaitContext> _waitContexts = new();
    private readonly ConcurrentDictionary<Guid, Channel<ServerMessage>> _subscribers = new();
    
    public static async Task<GameNetworkHandler> ConnectAsync(string serverAddress, string botName, CancellationToken cancellationToken)
    {
        var handler = new GameNetworkHandler(serverAddress, botName);
        await handler.ConnectAsync(cancellationToken).ConfigureAwait(false);
        return handler;
    }
    
    public async Task EnqueueSendAsync(ClientMessage message, CancellationToken cancellationToken)
    {
        try
        {
            message.Version = VersionInfo.ServerVersion;
            await _messageQueueWriter.WriteAsync(message, cancellationToken).ConfigureAwait(false);
        }
        catch (ChannelClosedException)
        {
        }
    }
    
    public Task<ServerMessage> WaitForMessageOnceAsync(Predicate<ServerMessage> predicate, CancellationToken cancellationToken)
    {
        var defaultWaitTimeout = TimeSpan.FromSeconds(5);

        var timeoutCts = new CancellationTokenSource(defaultWaitTimeout);
        var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCts.Token);

        var id = Guid.NewGuid();
        var registration = linkedCts.Token.Register(() =>
        {
            if (_waitContexts.TryRemove(id, out var waitContext))
            {
                waitContext.Tcs.TrySetCanceled();
                waitContext.Dispose();
            }
        });

        var waitContext = new WaitContext(
            Id: id,
            Predicate: predicate,
            Tcs: new TaskCompletionSource<ServerMessage>(TaskCreationOptions.RunContinuationsAsynchronously),
            TimeoutCts: timeoutCts,
            LinkedCts: linkedCts,
            CancellationTokenRegistration: registration);

        _waitContexts[id] = waitContext;
        return waitContext.Tcs.Task;
    }
    
    public async IAsyncEnumerable<ServerMessage> ReadAllFromThisMomentAsync(
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var channel = Channel.CreateUnbounded<ServerMessage>();
        var id = Guid.NewGuid();
        _subscribers[id] = channel;

        try
        {
            await foreach (var message in channel.Reader.ReadAllAsync(cancellationToken).ConfigureAwait(false))
            {
                yield return message;
            }
        }
        finally
        {
            _subscribers.TryRemove(id, out _);
            channel.Writer.TryComplete();
        }
    }
    
    private GameNetworkHandler(string serverAddress, string botName)
    {
        _grpcChannel = GrpcChannel.ForAddress(serverAddress);
        var client = new GameService.GameService.GameServiceClient(_grpcChannel.Intercept(new LoggingInterceptor()));

        var call = client.GameStream();
        _streamWriter = call.RequestStream;
        _streamReader = call.ResponseStream;
        ClientId = GenerateClientId(botName);
        
        var messageChannel = Channel.CreateUnbounded<ClientMessage>();
        _messageQueueReader = messageChannel.Reader;
        _messageQueueWriter = messageChannel.Writer;
        
        _readTask = RunReadLoopAsync(_cts.Token);
        _writeTask = RunWriteLoopAsync();
    }
    

    private async Task ConnectAsync(CancellationToken cancellationToken)
    {
        Console.WriteLine($"Connecting with clientId: {ClientId}");

        var connectMessage = new ClientMessage
        {
            Connect = new ConnectRequest
            {
                ClientId = ClientId
            }
        };
        var waitForConnectResponseTask = WaitForMessageOnceAsync(
            message => message.MessageCase is ServerMessage.MessageOneofCase.Connect,
            cancellationToken);

        await EnqueueSendAsync(connectMessage, cancellationToken).ConfigureAwait(false);

        var responseMessage = await waitForConnectResponseTask.ConfigureAwait(false);
        if (!responseMessage.Connect.Success)
        {
            throw new Exception($"Failed to connect to server: {responseMessage.Connect.ErrorMessage}");
        }
    }

    private async Task RunReadLoopAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var message in _streamReader.ReadAllAsync(cancellationToken))
            {
                if (message.MessageCase is ServerMessage.MessageOneofCase.Shutdown)
                {
                    throw new OperationCanceledException("Server is shutting down");
                }

                if (message.MessageCase is ServerMessage.MessageOneofCase.Error)
                {
                    throw new Exception($"Server error: {message.Error.Message}");
                }

                var contextsToRemove = new List<Guid>();
                foreach (var waitContext in _waitContexts.Values)
                {
                    if (waitContext.Predicate(message))
                    {
                        waitContext.Tcs.TrySetResult(message);
                        contextsToRemove.Add(waitContext.Id);
                    }
                }
                
                foreach (var contextId in contextsToRemove)
                {
                    if (_waitContexts.TryRemove(contextId, out var removed))
                    {
                        removed.Dispose();
                    }
                }

                foreach (var subscriber in _subscribers.Values)
                {
                    subscriber.Writer.TryWrite(message);
                }
            }
        }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled && cancellationToken.IsCancellationRequested)
        {
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        finally
        {
            foreach (var subscriber in _subscribers.Values)
            {
                subscriber.Writer.TryComplete();
            }
        }
    }
    
    private async Task RunWriteLoopAsync()
    {
        try
        {
            while (await _messageQueueReader.WaitToReadAsync().ConfigureAwait(false))
            {
                while (_messageQueueReader.TryRead(out var message))
                {
                    await _streamWriter.WriteAsync(message).ConfigureAwait(false);
                }
            }
        }
        catch
        {
        }
    }
    
    
    private static string GenerateClientId(string botName)
    {
        return $"{botName}Bot_{Guid.NewGuid().ToString("N")[..8]}";
    }

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;

        await EnqueueSendAsync(new ClientMessage
        {
            Disconnect = new DisconnectRequest()
        }, CancellationToken.None).ConfigureAwait(false);

        _messageQueueWriter.Complete();
        await _writeTask.WaitAsync(TimeSpan.FromSeconds(5)).ConfigureAwait(false);
        await _cts.CancelAsync().ConfigureAwait(false);
        await _readTask.WaitAsync(TimeSpan.FromSeconds(5)).ConfigureAwait(false);
        _cts.Dispose();
        _grpcChannel.Dispose();
    }
    
    private sealed record WaitContext(
        Guid Id,
        Predicate<ServerMessage> Predicate,
        TaskCompletionSource<ServerMessage> Tcs,
        CancellationTokenSource TimeoutCts,
        CancellationTokenSource LinkedCts,
        CancellationTokenRegistration CancellationTokenRegistration)
    {
        public void Dispose()
        {
            CancellationTokenRegistration.Dispose();
            LinkedCts.Dispose();
            TimeoutCts.Dispose();
        }
    }
}